#!/usr/bin/env node
// Read-only SonarQube Cloud main-branch diagnostics; never labels a failure PASS.
// Execute only inside a trusted same-repository PR job with contents:read.
const project='suvorovpravda_Pravda-Bot';
const requests=[
  ['issues','https://sonarcloud.io/api/issues/search',{componentKeys:project,branch:'main',resolved:'false',ps:'100'}],
  ['new_issues','https://sonarcloud.io/api/issues/search',{componentKeys:project,branch:'main',resolved:'false',sinceLeakPeriod:'true',ps:'100'}],
  ['duplication','https://sonarcloud.io/api/measures/component_tree',{component:project,metricKeys:'new_duplicated_lines_density,new_duplicated_lines,new_lines',qualifiers:'FIL',ps:'500'}],
  ['aggregate','https://sonarcloud.io/api/measures/component',{component:project,metricKeys:'new_duplicated_lines_density,new_duplicated_lines,new_lines,duplicated_lines_density,duplicated_lines'}],
];
// SonarCloud returns at most one page per request. A partial response must never
// be reported as a complete historical issue inventory.
export async function collectIssues(endpoint,params,fetcher=fetch){
  const issues=[];
  const seenKeys=new Set();
  let expected=null;
  for(let page=1;page<=100;page++){
    const url=new URL(endpoint);
    for(const [key,value] of Object.entries(params))url.searchParams.set(key,value);
    url.searchParams.set('p',String(page));
    const response=await fetcher(url,{headers:{Accept:'application/json'},signal:AbortSignal.timeout(20000)});
    if(!response.ok)throw Error('SonarCloud HTTP '+response.status+' on page '+page);
    const data=await response.json();
    if(!Array.isArray(data.issues)||!Number.isSafeInteger(data.total)||data.total<0)
      throw Error('invalid SonarCloud issue schema on page '+page);
    // An HTTP 200 can still contain an unexpected server-side page. Reject it
    // rather than silently treating repeated or skipped offsets as complete.
    if(data.paging){
      if(data.paging.pageIndex!==page||data.paging.pageSize!==Number(params.ps)||data.paging.total!==data.total)
        throw Error('SonarCloud paging metadata mismatch on page '+page);
    }
    if(expected===null)expected=data.total;
    if(data.total!==expected)throw Error('SonarCloud total changed during pagination');
    if(issues.length+data.issues.length>expected)throw Error('SonarCloud issue count exceeds total');
    // Sonar pages use fixed offsets (p=2 begins at item 101). A short
    // non-final page silently skips records even when the final count matches.
    const pageSize=Number(params.ps);
    if(!Number.isSafeInteger(pageSize)||pageSize<1)throw Error('invalid SonarCloud page size');
    const remaining=expected-issues.length;
    if(data.issues.length!==Math.min(pageSize,remaining))
      throw Error('SonarCloud page length does not match fixed pagination offsets on page '+page);
    for(const issue of data.issues){
      if(typeof issue.key!=='string'||!issue.key)throw Error('SonarCloud issue missing key on page '+page);
      if(seenKeys.has(issue.key))throw Error('SonarCloud duplicate issue key across pages: '+issue.key);
      seenKeys.add(issue.key);
      issues.push(issue);
    }
    if(issues.length===expected)return {issues,total:expected};
    if(data.issues.length===0)throw Error('SonarCloud returned empty incomplete page');
  }
  throw Error('SonarCloud issue inventory exceeds 100 pages');
}

export async function main(fetcher=fetch){
let requiredDiagnosticsAvailable=true;
for(const [kind,endpoint,params] of requests){
  const url=new URL(endpoint);
  for(const [key,value] of Object.entries(params))url.searchParams.set(key,value);
  try{
    if(kind==='issues'||kind==='new_issues'){
      const data=await collectIssues(endpoint,params,fetcher);
      console.log('SONAR_MAIN_'+kind.toUpperCase()+'_HTTP=200');
      console.log('SONAR_MAIN_'+kind.toUpperCase()+'_TOTAL='+data.total);
      for(const q of data.issues)console.log('SONAR_MAIN_'+(kind==='new_issues'?'NEW_ISSUE':'ISSUE')+'='+JSON.stringify({key:q.key,rule:q.rule,component:q.component,line:q.line??q.textRange?.startLine??null,severity:q.severity,type:q.type,impacts:q.impacts,message:String(q.message??'').slice(0,350)}));
      continue;
    }
    const response=await fetcher(url,{headers:{Accept:'application/json'},signal:AbortSignal.timeout(20000)});
    console.log('SONAR_MAIN_'+kind.toUpperCase()+'_HTTP='+response.status);
    if(!response.ok){console.log('SONAR_MAIN_'+kind.toUpperCase()+'_UNAVAILABLE');requiredDiagnosticsAvailable=false;continue;}
    const data=await response.json();
    if(kind==='aggregate'){
      if(!data.component||!Array.isArray(data.component.measures))throw Error('invalid SonarCloud aggregate measures schema');
      console.log('SONAR_MAIN_AGGREGATE='+JSON.stringify(data.component?.measures??[]));
    }else{
      if(!Array.isArray(data.components)||!data.paging||!Number.isSafeInteger(data.paging.total)||data.paging.total<0)throw Error('invalid SonarCloud duplication measures schema');
      const list=data.components;
      console.log('SONAR_MAIN_COMPONENTS_TOTAL='+data.paging?.total);
      for(const q of list.slice(0,3))console.log('SONAR_MAIN_COMPONENT_SAMPLE='+JSON.stringify({key:q.key,measures:q.measures}));
      const qualified=list.filter(q=>(q.measures??[]).some(m=>m.metric==='new_duplicated_lines' && Number(m.period?.value??m.periods?.[0]?.value??m.value??0)>0));
      for(const q of qualified.slice(0,100))console.log('SONAR_MAIN_DUPLICATION='+JSON.stringify({key:q.key,measures:q.measures.map(m=>({metric:m.metric,value:m.value,period:m.period?.value??m.periods?.[0]?.value}))}));
    }
  }catch(e){
    console.log('SONAR_MAIN_'+kind.toUpperCase()+'_UNAVAILABLE='+String(e?.message||e?.name||'error'));
    requiredDiagnosticsAvailable=false;
  }
}
return requiredDiagnosticsAvailable;
}
if(process.argv[1] && import.meta.url===new URL('file://'+process.argv[1]).href){
  if(!await main())process.exitCode=1;
}
