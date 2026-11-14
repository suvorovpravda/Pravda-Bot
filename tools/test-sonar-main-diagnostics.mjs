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
  return {ok:true,json:async()=>({total:102,issues:Array.from({length:page===1?100:2},(_,i)=>({rule:'javascript:S1234',component:'test:file.js',key:String((page-1)*100+i)}))})};
};
const result=await collectIssues(endpoint,params,fetcher);
assert.equal(result.total,102);
assert.equal(result.issues.length,102);
assert.deepEqual(seen,[1,2]);

await assert.rejects(collectIssues(endpoint,params,async url=>({
  ok:true,json:async()=>({total:102,issues:url.searchParams.get('p')==='1'?Array.from({length:100},(_,i)=>({rule:'javascript:S1234',component:'test:file.js',key:String((Number(url.searchParams.get('p'))-1)*100+i)})):[]})
})),/page length does not match fixed pagination offsets/);
await assert.rejects(collectIssues(endpoint,params,async url=>({
  ok:url.searchParams.get('p')==='1',status:503,
  json:async()=>({total:102,issues:Array.from({length:100},(_,i)=>({rule:'javascript:S1234',component:'test:file.js',key:String((Number(url.searchParams.get('p'))-1)*100+i)}))})
})),/HTTP 503 on page 2/);
await assert.rejects(collectIssues(endpoint,params,async url=>({
  ok:true,json:async()=>({total:url.searchParams.get('p')==='1'?102:103,issues:Array.from({length:100},(_,i)=>({rule:'javascript:S1234',component:'test:file.js',key:String((Number(url.searchParams.get('p'))-1)*100+i)}))})
})),/total changed/);
// Sonar uses fixed page offsets. Fifty issues on page 1 and 52 on page 2
// must not masquerade as all 102 unique issues: entries 51-100 were skipped.
await assert.rejects(collectIssues(endpoint,params,async url=>({
  ok:true,json:async()=>({total:102,issues:Array.from({length:url.searchParams.get('p')==='1'?50:52},(_,i)=>({rule:'javascript:S1234',component:'test:file.js',key:String((url.searchParams.get('p')==='1'?0:100)+i)}))})
})),/page length does not match fixed pagination offsets/);
// A successful HTTP response with the wrong page metadata must not be trusted.
await assert.rejects(collectIssues(endpoint,params,async url=>({
  ok:true,json:async()=>({total:102,paging:{pageIndex:1,pageSize:100,total:102},
    issues:Array.from({length:url.searchParams.get('p')==='1'?100:2},(_,i)=>({rule:'javascript:S1234',component:'test:file.js',key:String((Number(url.searchParams.get('p'))-1)*100+i)}))})
})),/paging metadata mismatch on page 2/);
// A green job must not imply that historical issues were retrieved when the API failed.
await assert.rejects(collectIssues(endpoint,params,async url=>({
  ok:true,json:async()=>({total:102,issues:Array.from({length:url.searchParams.get('p')==='1'?100:2},(_,i)=>({rule:'javascript:S1234',component:'test:file.js',key:String(i)}))})
})),/duplicate issue key across pages/);
await assert.rejects(collectIssues(endpoint,params,async()=>({
  ok:true,json:async()=>({total:1,issues:[{}]})
})),/issue missing key/);
// A response with issue keys but no rule/file is not a usable Sonar triage inventory.
await assert.rejects(collectIssues(endpoint,params,async()=>({
  ok:true,json:async()=>({total:1,issues:[{key:'k1',component:'test:file.js'}]})
})),/missing rule or component/);
await assert.rejects(collectIssues(endpoint,params,async()=>({
  ok:true,json:async()=>({total:1,issues:[{key:'k1',rule:'javascript:S1234'}]})
})),/missing rule or component/);
const originalLog=console.log;
try{
  console.log=()=>{};
  const failed=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:false,status:503};
    return {ok:true,status:200,json:async()=>({component:{measures:[]},components:[],paging:{total:0,pageIndex:1,pageSize:500}})};
  });
  assert.equal(failed,false,'required issue API failure must fail the diagnostic');
  const succeeded=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    return {ok:true,status:200,json:async()=>({component:{measures:[]},components:[],paging:{total:0,pageIndex:1,pageSize:500}})};
  });
  assert.equal(succeeded,true,'complete issue inventory must pass');
  // A 501-file duplication inventory must retrieve the second page.
  const pages=[];
  const paginated=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component')return {ok:true,status:200,json:async()=>({component:{measures:[]}})};
    const page=Number(url.searchParams.get('p'));
    pages.push(page);
    return {ok:true,status:200,json:async()=>({paging:{total:501,pageIndex:page,pageSize:500},components:Array.from({length:page===1?500:1},(_,i)=>({rule:'javascript:S1234',component:'test:file.js',key:String((page-1)*500+i),measures:[]}))})};
  });
  assert.equal(paginated,true,'all duplication pages must be retrieved');
  assert.deepEqual(pages,[1,2]);
  // Repeated component identities across fixed-offset pages must not be counted
  // as a complete 501-file inventory (the omitted file could carry duplication).
  const repeatedComponents=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component')return {ok:true,status:200,json:async()=>({component:{measures:[]}})};
    const page=Number(url.searchParams.get('p'));
    return {ok:true,status:200,json:async()=>({paging:{total:501,pageIndex:page,pageSize:500},
      components:Array.from({length:page===1?500:1},(_,i)=>({rule:'javascript:S1234',component:'test:file.js',key:String(page===1?i:0),measures:[]}))})};
  });
  assert.equal(repeatedComponents,false,'duplicate component keys across pages must fail closed');
  const missingComponentKey=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component')return {ok:true,status:200,json:async()=>({component:{measures:[]}})};
    return {ok:true,status:200,json:async()=>({paging:{total:1,pageIndex:1,pageSize:500},components:[{measures:[]}]})};
  });
  assert.equal(missingComponentKey,false,'component without stable identity must fail closed');
  // An HTTP 200 with a nonnumeric duplicated-lines value must not claim zero duplicates.
  const invalidDuplicationMetric=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component')return {ok:true,status:200,json:async()=>({component:{measures:[]}})};
    return {ok:true,status:200,json:async()=>({paging:{total:1,pageIndex:1,pageSize:500},components:[
      {key:'broken-file',measures:[{metric:'new_duplicated_lines',value:'not-a-number'}]}
    ]})};
  });
  assert.equal(invalidDuplicationMetric,false,'malformed duplication metric must fail closed');
  // A malformed response must never print the success marker before failing.
  const diagnosticLogs=[];
  console.log=(...args)=>diagnosticLogs.push(args.join(' '));
  const malformedMetricLogged=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component')return {ok:true,status:200,json:async()=>({component:{measures:[]}})};
    return {ok:true,status:200,json:async()=>({paging:{total:1,pageIndex:1,pageSize:500},components:[
      {key:'broken-file',measures:[{metric:'new_duplicated_lines',value:'NaN'}]}
    ]})};
  });
  assert.equal(malformedMetricLogged,false);
  assert.ok(!diagnosticLogs.includes('SONAR_MAIN_DUPLICATION_HTTP=200'),
    'invalid duplication inventory must not emit a success marker');
  console.log=()=>{};
  // A file without measures must not be treated as a verified zero-duplication file.
  const omittedFileMeasures=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component')return {ok:true,status:200,json:async()=>({component:{measures:[]}})};
    return {ok:true,status:200,json:async()=>({paging:{total:1,pageIndex:1,pageSize:500},components:[
      {key:'file-with-unknown-duplication'}
    ]})};
  });
  assert.equal(omittedFileMeasures,false,'omitted file measures must fail closed');
  const malformedMeasures=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component')return {ok:true,status:200,json:async()=>({component:{measures:[]}})};
    return {ok:true,status:200,json:async()=>({paging:{total:1,pageIndex:1,pageSize:500},components:[
      {key:'broken-file',measures:{metric:'new_duplicated_lines',value:'12'}}
    ]})};
  });
  assert.equal(malformedMeasures,false,'malformed component measures must fail closed');
  // Missing pagination metadata can conceal a repeated first page.
  const missingPageMetadata=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component')return {ok:true,status:200,json:async()=>({component:{measures:[]}})};
    return {ok:true,status:200,json:async()=>({paging:{total:1},components:[{key:'file',measures:[]}]})};
  });
  assert.equal(missingPageMetadata,false,'missing duplication page index/size must fail closed');
  const shortDuplication=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component')return {ok:true,status:200,json:async()=>({component:{measures:[]}})};
    return {ok:true,status:200,json:async()=>({paging:{total:501},components:[]})};
  });
  assert.equal(shortDuplication,false,'short duplication page must fail closed');
  // Missing coverage/duplication measures must not leave a misleading green diagnostic.
  const missingMeasures=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component_tree')return {ok:false,status:503};
    return {ok:true,status:200,json:async()=>({component:{measures:[]}})};
  });
  assert.equal(missingMeasures,false,'unavailable duplication measures must fail the diagnostic');
  const missingAggregate=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component')return {ok:false,status:503};
    return {ok:true,status:200,json:async()=>({components:[],paging:{total:0}})};
  });
  assert.equal(missingAggregate,false,'unavailable aggregate measures must fail the diagnostic');
  // HTTP 200 with malformed measures must fail closed, not be logged as zero findings.
  const malformedDuplication=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component_tree')return {ok:true,status:200,json:async()=>({paging:{total:4}})};
    return {ok:true,status:200,json:async()=>({component:{measures:[]}})};
  });
  assert.equal(malformedDuplication,false,'malformed duplication HTTP 200 must fail');
  const malformedAggregate=await main(async url=>{
    if(url.pathname==='/api/issues/search')return {ok:true,json:async()=>({total:0,issues:[]})};
    if(url.pathname==='/api/measures/component')return {ok:true,status:200,json:async()=>({component:{}})};
    return {ok:true,status:200,json:async()=>({components:[],paging:{total:0}})};
  });
  assert.equal(malformedAggregate,false,'malformed aggregate HTTP 200 must fail');
}finally{
  console.log=originalLog;
}
console.log('Sonar main pagination and required-inventory failure regression OK');

// The historical main inventory must be checked on every main SHA, not only PRs
// that touch a limited set of paths. A PR-only workflow cannot monitor main.
const workflow=readFileSync(new URL('../.github/workflows/sonar-main-diagnostics.yml',import.meta.url),'utf8');
assert.ok(workflow.includes("  push:\n    branches: [main]"),'main pushes must trigger the diagnostic');
assert.ok(workflow.includes("  pull_request:\n    branches: [main]"),'PRs must retain diagnostics');
assert.ok(!workflow.includes('    paths:') && !workflow.includes('    paths-ignore:'),'path filters may not skip main commits');
assert.ok(workflow.includes("github.event_name == 'push'"),'main push job must not be skipped by PR-only guard');
assert.ok(workflow.includes('run: node tools/test-sonar-main-diagnostics.mjs'),'offline regression must run in CI');

// PR checks must test the exact submitted head, not GitHub's synthetic merge SHA.
assert.ok(workflow.includes("github.event.pull_request.head.sha || github.sha"),
  'PR checkout must use exact head SHA while push uses event SHA');
