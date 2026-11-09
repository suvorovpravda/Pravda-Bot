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
for(const [kind,endpoint,params] of requests){
  const url=new URL(endpoint);
  for(const [key,value] of Object.entries(params))url.searchParams.set(key,value);
  try{
    const response=await fetch(url,{headers:{Accept:'application/json'},signal:AbortSignal.timeout(20000)});
    console.log('SONAR_MAIN_'+kind.toUpperCase()+'_HTTP='+response.status);
    if(!response.ok){console.log('SONAR_MAIN_'+kind.toUpperCase()+'_UNAVAILABLE');continue;}
    const data=await response.json();
    if(kind==='issues'||kind==='new_issues'){
      if(!Array.isArray(data.issues))throw Error('unexpected response schema');
      console.log('SONAR_MAIN_ISSUES_TOTAL='+data.total);
      for(const q of data.issues)console.log('SONAR_MAIN_'+(kind==='new_issues'?'NEW_ISSUE':'ISSUE')+'='+JSON.stringify({key:q.key,rule:q.rule,component:q.component,line:q.line??q.textRange?.startLine??null,severity:q.severity,type:q.type,impacts:q.impacts,message:String(q.message??'').slice(0,350)}));
    }else if(kind==='aggregate'){
      console.log('SONAR_MAIN_AGGREGATE='+JSON.stringify(data.component?.measures??[]));
    }else{
      const list=Array.isArray(data.components)?data.components:[];
      console.log('SONAR_MAIN_COMPONENTS_TOTAL='+data.paging?.total);
      for(const q of list.slice(0,3))console.log('SONAR_MAIN_COMPONENT_SAMPLE='+JSON.stringify({key:q.key,measures:q.measures}));
      const qualified=list.filter(q=>(q.measures??[]).some(m=>m.metric==='new_duplicated_lines' && Number(m.periods?.[0]?.value??m.value??0)>0));
      for(const q of qualified.slice(0,100))console.log('SONAR_MAIN_DUPLICATION='+JSON.stringify({key:q.key,measures:q.measures.map(m=>({metric:m.metric,value:m.value,period:m.period?.value}))}));
    }
  }catch(e){console.log('SONAR_MAIN_'+kind.toUpperCase()+'_UNAVAILABLE='+String(e?.name||'error'));}
}
