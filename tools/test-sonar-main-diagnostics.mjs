#!/usr/bin/env node
import assert from 'node:assert/strict';
import {collectIssues} from './sonar-main-diagnostics.mjs';

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
console.log('Sonar main pagination regression OK');
