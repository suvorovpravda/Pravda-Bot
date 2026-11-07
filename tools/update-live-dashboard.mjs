#!/usr/bin/env node
/**
 * Observability only. Builds the near-live Markdown SVG/JSON from public GitHub
 * workflow/issue/PR state and verified historical rows in the central guide.
 * NEVER infers production readiness, trading availability, or a security score.
 */
import assert from "node:assert/strict";
import {readFileSync, mkdirSync, writeFileSync, existsSync} from "node:fs";
import {dirname, join} from "node:path";

export const WORKFLOWS = Object.freeze([
  {id:"rust.yml", name:"RUST / WINDOWS + LINUX", kind:"required"},
  {id:"windows-package.yml", name:"MSI / NSIS INSTALLER", kind:"required"},
  {id:"codeql.yml", name:"CODEQL / ANALYSE", kind:"required"},
  {id:"rustsec-audit.yml", name:"RUSTSEC / OBSERVATION", kind:"observe"}
]);

const TAG = "PRAVDA-STATUS-v1";
const OUT_SVG = "docs/assets/pravda-live-status.svg";
const OUT_JSON = "docs/assets/pravda-live-status.json";
const guidePath = "docs/PRAVDA-GUIDE.md";
const safe = value => String(value).replace(/&/g,"&amp;").replace(/</g,"&lt;")
  .replace(/>/g,"&gt;").replace(/"/g,"&quot;").replace(/'/g,"&apos;");
const pct = (a,b) => b ? Math.round(a*100/b) : 0;

export function parseHistorical(markdown) {
  const rows = [];
  for(const line of markdown.split(/\r?\n/)) {
    if(!/^\|\s*\*\*P[012]\*\*\s*\|/.test(line)) continue;
    const priority = line.match(/^\|\s*\*\*(P[012])\*\*/)?.[1];
    const fraction = line.match(/\b(\d+)\s*\/\s*(\d+)\b/);
    const percentage = line.match(/\b(\d+)\s*%/);
    assert.ok(priority && fraction && percentage, "Historical guide has an incomplete P0-P2 row");
    const done=Number(fraction[1]), total=Number(fraction[2]), published=Number(percentage[1]);
    assert.ok(total>0 && done>=0 && done<=total);
    assert.equal(pct(done,total),published,"Historical guide metric must be consistent");
    rows.push({priority,done,total,percentage:published});
  }
  assert.equal(rows.length,9,"Exactly nine verified historical guide workstreams required");
  const groups=["P0","P1","P2"].map(priority=>{
    const selected=rows.filter(row=>row.priority===priority);
    const done=selected.reduce((n,row)=>n+row.done,0);
    const total=selected.reduce((n,row)=>n+row.total,0);
    return {priority,done,total,percentage:pct(done,total)};
  });
  const done=rows.reduce((n,row)=>n+row.done,0),total=rows.reduce((n,row)=>n+row.total,0);
  return {rows,groups,done,total,percentage:pct(done,total),origin:"docs/PRAVDA-GUIDE.md"};
}

export function selectedRun(workflowRuns, sourceSha, observeOnly=false) {
  assert.match(sourceSha,/^[0-9a-f]{40}$/,"A concrete source commit is required");
  const candidates = Array.isArray(workflowRuns) ? workflowRuns
    .filter(r=>r?.head_branch==="main" && typeof r.head_sha==="string")
    .sort((a,b)=>new Date(b.created_at||0)-new Date(a.created_at||0)): [];
  const run=candidates[0];
  if(!run) return {state:"NOT_RUN",sha:null,url:null,observedAt:null};
  const sha=run.head_sha.toLowerCase();
  const sameSha=sha===sourceSha.toLowerCase();
  const state= !sameSha ? "STALE_SHA" :
    run.status!=="completed" ? "RUNNING" :
    observeOnly ? "OBSERVE_ONLY" :
    run.conclusion==="success" ? "PASS" :
    run.conclusion==="failure" ? "FAIL" :
    "INCOMPLETE";
  const url = typeof run.html_url === "string" && /^https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/actions\/runs\/\d+$/.test(run.html_url)
    ?run.html_url:null;
  return {state,sha:sha.slice(0,10),url,observedAt:run.updated_at||run.created_at||null,
    // A nonblocking audit is not a clean bill of health, even if CI says success.
    auditMode:observeOnly?"nonblocking":undefined};
}

export function buildSnapshot({guide,runs,prs,issues,sourceSha}) {
  const history=parseHistorical(guide);
  assert.ok(Array.isArray(prs) && Array.isArray(issues));
  const statuses = WORKFLOWS.map(w=>({
    id:w.id,name:w.name,kind:w.kind,...selectedRun(runs[w.id]||[],sourceSha,w.kind==="observe")
  }));
  const required=statuses.filter(s=>s.kind==="required");
  const passed=required.filter(s=>s.state==="PASS").length;
  const openPrs=prs.filter(p=>p.state==="open").length;
  const openIssues=issues.filter(i=>i.state==="open" && !i.pull_request).length;
  const lastEvent= statuses.map(s=>s.observedAt).filter(Boolean)
    .sort((a,b)=>Date.parse(b)-Date.parse(a))[0]||null;
  return {
    schema:TAG,source_sha:sourceSha,latest_evidence_at:lastEvent,
    historical:history,required_checks:{passed,total:required.length},
    workflows:statuses,open_prs:openPrs,open_issues:openIssues,
    production:"NO-GO",trading_live:"OFF",signed_updater:"OFF",
    security_note:"RustSec observe-only is never treated as PASS",
    data_source:"GitHub Actions API + GitHub Issues/PR API + central guide",
    polling:"GitHub scheduled best effort / 10 min + selected completed workflows",
  };
}

function nodeText(x,y,text,size=20,color="#F6F7FF",weight=600,extra="") {
  return '<text x="'+x+'" y="'+y+'" fill="'+color+'" font-family="Segoe UI,Arial,sans-serif" font-size="'+size+'" font-weight="'+weight+'" '+extra+'>'+safe(text)+'</text>';
}
function panel(x,y,w,h,fill="#151B36",stroke="#363F65") {
  return '<rect x="'+x+'" y="'+y+'" width="'+w+'" height="'+h+'" rx="20" fill="'+fill+'" stroke="'+stroke+'"/>';
}
function ring(x,y,p,color,r=47) {
  const c=2*Math.PI*r;
  return '<circle cx="'+x+'" cy="'+y+'" r="'+r+'" fill="none" stroke="#303956" stroke-width="14"/>'+
  '<circle cx="'+x+'" cy="'+y+'" r="'+r+'" fill="none" stroke="'+color+'" stroke-width="14" stroke-linecap="round" stroke-dasharray="'+(c*p/100).toFixed(2)+' '+(c*(100-p)/100).toFixed(2)+'" transform="rotate(-90 '+x+' '+y+')"/>';
}
function stateColor(state) {
  return ({PASS:"#54DFC0",RUNNING:"#51D9FB",FAIL:"#FF668D",
    OBSERVE_ONLY:"#FFCD76",STALE_SHA:"#B39AFC",NOT_RUN:"#F9AB70",INCOMPLETE:"#FFCD76"})[state]||"#B1BDDD";
}
export function renderSvg(snapshot) {
  const h=snapshot.historical, checks=snapshot.required_checks;
  const rows=snapshot.workflows;
  const colors=["#50D9FB","#FF5597","#A78BFF"];
  const blocks=[];
  blocks.push('<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="485" viewBox="0 0 1600 485" role="img">');
  blocks.push('<title>Pravda-Bot Live Pulse — GitHub workflow status, historical milestone progress and release NO-GO</title>');
  blocks.push('<desc>Source SHA '+safe(snapshot.source_sha.slice(0,10))+'. Historic milestone percentage '+h.percentage+
    ' percent. Latest three required CI workflow outcomes '+checks.passed+' over '+checks.total+'. Production NO-GO.</desc>');
  blocks.push('<defs><linearGradient id="livebg" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#181C3E"/><stop offset=".58" stop-color="#0C1328"/><stop offset="1" stop-color="#18132E"/></linearGradient></defs>');
  blocks.push('<rect width="1600" height="485" rx="24" fill="url(#livebg)"/>');
  blocks.push(panel(34,27,1532,435,"#0E1530","#455078"));
  blocks.push(nodeText(68,70,"●  PRAVDA / LIVE PULSE",29,"#F3F5FF",800,'letter-spacing="1.4"'));
  blocks.push(nodeText(69,99,"Synchronisation planifiée toutes les 10 min — exécution différée possible sur GitHub Actions",15,"#A6B3D4",500));
  blocks.push(nodeText(1278,73,"NO-GO",33,"#FF668D",800));
  blocks.push(nodeText(1280,101,"PRODUCTION / MANUAL GATE",13,"#B8BDD6",650));
  blocks.push(panel(66,128,348,169));blocks.push(panel(429,128,348,169));
  blocks.push(panel(792,128,345,169));blocks.push(panel(1152,128,378,169));
  blocks.push(ring(152,216,h.percentage,"#A88BFF"));
  blocks.push(nodeText(120,224,h.percentage+"%",27,"#FFFFFF",800));
  blocks.push(nodeText(221,186,"HISTORICAL",17,"#9DAFFF",700));
  blocks.push(nodeText(221,215,h.done+" / "+h.total+" jalons",19,"#F3F5FF",750));
  blocks.push(nodeText(221,242,"Guide P0–P2",14,"#ABB4D0",500));
  blocks.push(ring(513,216,pct(checks.passed,checks.total),"#51D9FB"));
  blocks.push(nodeText(486,224,checks.passed+"/"+checks.total,26,"#FFFFFF",800));
  blocks.push(nodeText(584,185,"WORKFLOWS CI",17,"#9DAFFF",700));
  blocks.push(nodeText(584,213,"au SHA source",15,"#F3F5FF",600));
  blocks.push(nodeText(584,239,"Pas un score release",14,"#ABB4D0",500));
  blocks.push(nodeText(818,186,"PULL REQUESTS",17,"#ABB4D0",700));
  blocks.push(nodeText(818,253,snapshot.open_prs.toString(),73,"#FF579C",800));
  blocks.push(nodeText(1175,186,"ISSUES OUVERTES",17,"#ABB4D0",700));
  blocks.push(nodeText(1175,253,snapshot.open_issues.toString(),73,"#6BDDFC",800));
  for(let i=0;i<4;i++){
    const row=rows[i],x=73+i*377,y=331;
    const color=stateColor(row.state);
    blocks.push(panel(x,y,355,77,"#1D2443","#41476E"));
    blocks.push('<circle cx="'+(x+18)+'" cy="'+(y+21)+'" r="5" fill="'+color+'"/>');
    blocks.push(nodeText(x+33,y+27,row.name,14,"#F3F5FF",750));
    blocks.push(nodeText(x+17,y+54,row.state.replaceAll("_"," ")+" · "+(row.sha||"—"),15,color,700));
  }
  const update=snapshot.latest_evidence_at ? new Date(snapshot.latest_evidence_at).toISOString().replace("T"," ").slice(0,16)+" UTC" : "aucune preuve reçue";
  blocks.push(nodeText(73,448,"LATEST EVIDENCE  "+update+"   ·   SOURCE SHA  "+snapshot.source_sha.slice(0,10),14,"#A6B5D5",650));
  blocks.push(nodeText(1316,448,"LIVE ORDERS  OFF",14,"#65DDBA",730));
  blocks.push("</svg>");
  return blocks.join("\n")+"\n";
}

async function api(path) {
  const token=process.env.GITHUB_TOKEN;
  assert.ok(token, "GITHUB_TOKEN required for live refresh; no anonymous fallback to incomplete data");
  const origin="https://api.github.com";
  const response=await fetch(origin+path,{
    headers:{"Accept":"application/vnd.github+json","Authorization":"Bearer "+token,
      "X-GitHub-Api-Version":"2022-11-28","User-Agent":"Pravda-Status-Monitor"}
  });
  if(!response.ok) throw Error("GitHub API "+response.status+" for "+path.split("?")[0]);
  return response.json();
}

async function loadGithub(repo, branch) {
  assert.match(repo,/^[\w.-]+\/[\w.-]+$/);
  assert.equal(branch,"main","Live status always reads canonical main");
  const encodedRepo=repo.split("/").map(encodeURIComponent).join("/");
  const prefix="/repos/"+encodedRepo;
  const commits=await api(prefix+"/commits?sha=main&per_page=35");
  assert.ok(commits.length,"Cannot infer source SHA without canonical commit history");
  const source=commits.find(c=>!/^docs\(live\):/.test(c.commit?.message||""));
  assert.ok(source?.sha && /^[a-f0-9]{40}$/.test(source.sha),"No verifiable non-status main source commit");
  const requests=[
    ...WORKFLOWS.map(w=>api(prefix+"/actions/workflows/"+encodeURIComponent(w.id)+"/runs?branch=main&per_page=14")
      .then(x=>[w.id,x.workflow_runs||[]])),
    api(prefix+"/pulls?state=open&per_page=100").then(x=>["prs",x]),
    api(prefix+"/issues?state=open&per_page=100").then(x=>["issues",x])
  ];
  const entries=await Promise.all(requests);
  const data=Object.fromEntries(entries);
  return {sourceSha:source.sha,runs:Object.fromEntries(WORKFLOWS.map(w=>[w.id,data[w.id]])),
    prs:data.prs,issues:data.issues};
}

function stableJson(value){return JSON.stringify(value,null,2)+"\n"}
function put(path,content) {
  mkdirSync(dirname(path),{recursive:true});
  if(existsSync(path) && readFileSync(path,"utf8")===content) return false;
  writeFileSync(path,content,"utf8");
  return true;
}
function runTests(){
  // Regression for SonarCloud workflow_run trust boundary: a workflow run from
  // a fork whose branch is also "main" must not activate the write-capable job.
  const workflow=readFileSync(".github/workflows/live-docs.yml","utf8");
  const rootPermissions=workflow.split("\npermissions:\n")[1]?.split("\nconcurrency:\n")[0];
  assert.ok(rootPermissions,"Expected explicit top-level GitHub Actions permissions");
  assert.match(rootPermissions,/^  contents: read$/m,
    "Untrusted triggers must not inherit a write-capable GITHUB_TOKEN");
  assert.doesNotMatch(rootPermissions,/^  contents: write$/m);
  const updateJob=workflow.split("\n  update:\n")[1];
  assert.ok(updateJob,"Expected a separate privileged update job");
  const gate=updateJob.split("\n").find(line=>line.startsWith("    if: "))||"";
  assert.match(gate,/github\.event\.workflow_run\.head_branch\s*==\s*'main'/);
  assert.match(gate,/github\.event\.workflow_run\.head_repository\.full_name\s*==\s*github\.repository/,
    "Matching the branch name alone does not authenticate the source repository");
  assert.match(updateJob,/^    permissions:\n      contents: write$/m,
    "The write scope belongs to the trusted update job only");
  const guide=readFileSync(guidePath,"utf8"), h=parseHistorical(guide);
  assert.equal(h.rows.length,9);
  const sampleRun={head_branch:"main",head_sha:"a".repeat(40),status:"completed",conclusion:"success",
    created_at:"2026-11-06T21:00:00Z",updated_at:"2026-11-06T21:02:00Z"};
  const model=buildSnapshot({guide,sourceSha:"a".repeat(40),
    runs:Object.fromEntries(WORKFLOWS.map(w=>[w.id,[sampleRun]])),
    prs:[{state:"open"}],issues:[{state:"open"}, {state:"open",pull_request:{url:"x"}}]});
  assert.equal(model.required_checks.passed,3);
  assert.equal(model.workflows[3].state,"OBSERVE_ONLY");
  assert.equal(model.open_issues,1);
  assert.equal(selectedRun([sampleRun],"b".repeat(40)).state,"STALE_SHA");
  assert.equal(selectedRun([],"a".repeat(40)).state,"NOT_RUN");
  const svg=renderSvg(model);
  assert.match(svg,/NO-GO/);
  assert.match(svg,/OBSERVE ONLY/);
  assert.ok(svg.endsWith("</svg>\n"));
  console.log("PASS: live board percentages, SHA equality, RustSec observe-only, redaction and SVG layout");
}
async function main(){
  const args=process.argv.slice(2);
  if(args.includes("--test")) return runTests();
  const repo=process.env.GITHUB_REPOSITORY;
  const branch=process.env.GITHUB_REF_NAME||"main";
  const upstream=await loadGithub(repo,"main");
  const guide=readFileSync(guidePath,"utf8");
  const snapshot=buildSnapshot({...upstream,guide});
  const updated=[
    put(OUT_JSON,stableJson(snapshot)),
    put(OUT_SVG,renderSvg(snapshot))
  ];
  console.log(TAG+" | source="+snapshot.source_sha.slice(0,10)+
    " | CI="+snapshot.required_checks.passed+"/"+snapshot.required_checks.total+
    " | PR="+snapshot.open_prs+" | ISSUES="+snapshot.open_issues+
    " | changed="+updated.some(Boolean)+" | branch="+branch);
  if(snapshot.production!=="NO-GO")throw Error("Refuse unverified automatic GO");
}
if(import.meta.url===new URL(process.argv[1],"file://").href) {
  main().catch(error=>{console.error("LIVE_STATUS_REFRESH_FAILED:",error.message);process.exitCode=1;});
}
