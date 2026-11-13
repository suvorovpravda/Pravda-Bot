#!/usr/bin/env node
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {collectIssues,main} from './sonar-main-diagnostics.mjs';

const endpoint='https://sonarcloud.io/api/issues/search';
const params={componentKeys:'test',branch:'main',ps:'100'};
const seen=[];
const fetcher=async url=>{
  const page=Number(url.searchParams.get('p'));
  seen.push(page);
  assert.equal(url.searchParams.get('branch'),'main');
  return {ok:true,json:async()=>({total:102,issues:Array.from({length:page===1?100:2},(_,i)=>({key:String((page-1)*100+i)}))})};
};
const result=await collectIssues(endpoint,params,fetcher);
assert.equal(result.total,102);
assert.equal(result.issues.length,102);
assert.deepEqual(seen,[1,2]);

await assert.rejects(collectIssues(endpoint,params,async url=>({
  ok:true,json:async()=>({total:102,issues:url.searchParams.get('p')==='1'?Array(100).fill({}):[]})
})),/empty incomplete page/);
await assert.rejects(collectIssues(endpoint,params,async url=>({
  ok:url.searchParams.get('p')==='1',status:503,
  json:async()=>({total:102,issues:Array(100).fill({})})
})),/HTTP 503 on page 2/);
await assert.rejects(collectIssues(endpoint,params,async url=>({
  ok:true,json:async()=>({total:url.searchParams.get('p')==='1'?102:103,issues:Array(100).fill({})})
})),/total changed/);
// A green job must not imply that historical issues were retrieved when the API failed.
const originalLog=console.log;
try{
  console.log=()=>{};
  const failed=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:false,status:503};
    return {ok:true,status:200,json:async()=>({component:{measures:[]},components:[],paging:{total:0}})};
  });
  assert.equal(failed,false,'required issue API failure must fail the diagnostic');
  const succeeded=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    return {ok:true,status:200,json:async()=>({component:{measures:[]},components:[],paging:{total:0}})};
  });
  assert.equal(succeeded,true,'complete issue inventory must pass');
}finally{
  console.log=originalLog;
}
console.log('Sonar main pagination and required-inventory failure regression OK');

// The historical main inventory must be checked on every main SHA, not only PRs
// that touch a limited set of paths. A PR-only workflow cannot monitor main.
const workflow=readFileSync(new URL('../.github/workflows/sonar-main-diagnostics.yml',import.meta.url),'utf8');
assert.match(workflow,/^  push:\\n    branches: \\[main\\]/m,'main pushes must trigger the diagnostic');
assert.match(workflow,/^  pull_request:\\n    branches: \\[main\\]/m,'PRs must retain diagnostics');
assert.doesNotMatch(workflow,/^    paths(?:-ignore)?:/m,'no paths filters may skip main commits');
assert.match(workflow,/github\\.event_name == 'push'/,'main push job must not be skipped by PR-only guard');
assert.match(workflow,/run: node tools\\/test-sonar-main-diagnostics\\.mjs/,'offline regression must run in CI');
