#!/usr/bin/env node
// Read-only SonarQube Cloud main-branch diagnostics; never labels a failure PASS.
// Execute only inside a trusted same-repository PR job with contents:read.
const project='suvorovpravda_Pravda-Bot';
const requests=[
  ['issues','https://sonarcloud.io/api/issues/search',{componentKeys:project,branch:'main',resolved:'false',ps:'100'}],
  ['duplication','https://sonarcloud.io/api/measures/component_tree',{component:project,metricKeys:'new_duplicated_lines_density,new_duplicated_lines,new_lines',qualifiers:'FIL',ps:'500'}],
];
for(const [kind,endpoint,params] of requests){
  const url=new URL(endpoint);
  for(const [key,value] of Object.entries(params))url.searchParams.set(key,value);
  try{
    const response=await fetch(url,{headers:{Accept:'application/json'},signal:AbortSignal.timeout(20000)});
    console.log('SONAR_MAIN_'+kind.toUpperCase()+'_HTTP='+response.status);
    if(!response.ok){console.log('SONAR_MAIN_'+kind.toUpperCase()+'_UNAVAILABLE');continue;}
    const data=await response.json();
    if(kind==='issues'){
      if(!Array.isArray(data.issues))throw Error('unexpected response schema');
      console.log('SONAR_MAIN_ISSUES_TOTAL='+data.total);
      for(const q of data.issues)console.log('SONAR_MAIN_ISSUE='+JSON.stringify({key:q.key,rule:q.rule,component:q.component,line:q.line??q.textRange?.startLine??null,severity:q.severity,type:q.type,impacts:q.impacts,message:String(q.message??'').slice(0,350)}));
    }else{
      const list=Array.isArray(data.components)?data.components:[];
      console.log('SONAR_MAIN_COMPONENTS_TOTAL='+data.paging?.total);
      const qualified=list.filter(q=>(q.measures??[]).some(m=>Number(m.value??m.period?.value)>0));
      for(const q of qualified.slice(0,100))console.log('SONAR_MAIN_DUPLICATION='+JSON.stringify({key:q.key,measures:q.measures.map(m=>({metric:m.metric,value:m.value,period:m.period?.value}))}));
    }
  }catch(e){console.log('SONAR_MAIN_'+kind.toUpperCase()+'_UNAVAILABLE='+String(e?.name||'error'));}
}
