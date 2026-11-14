#!/usr/bin/env node
/**
 * Observability only. Builds the near-live Markdown SVG/JSON from public GitHub
 * workflow/issue/PR state and verified historical rows in the central guide.
 * NEVER infers production readiness, trading availability, or a security score.
 */
import assert from "node:assert/strict";
import {readFileSync, mkdirSync, writeFileSync, mkdtempSync, renameSync, rmSync, readdirSync} from "node:fs";
import {tmpdir} from "node:os";
import {dirname, join} from "node:path";

export const WORKFLOWS = Object.freeze([
  {id:"rust.yml", name:"RUST / WINDOWS + LINUX", kind:"required"},
  {id:"windows-package.yml", name:"MSI / NSIS INSTALLER", kind:"required"},
  {id:"codeql.yml", name:"CODEQL / ANALYSE", kind:"required"},
  {id:"rustsec-audit.yml", name:"RUSTSEC / AUDIT BLOQUANT", kind:"required"}
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
    // Nonblocking legacy workflow support; current RustSec workflow is blocking.
    auditMode:observeOnly?"nonblocking":undefined};
}

/** Audit the exact main SHA against its active GitHub ruleset and check runs.
 * Unknown API responses must never be presented as an effective protection.
 * This is read-only evidence, not a production-release authorization.
 */
export function evaluateGovernance(policy, checkRuns, sourceSha) {
  assert.match(sourceSha,/^[a-f0-9]{40}$/);
  const empty={state:"UNKNOWN",source_sha:sourceSha,policy_active:false,
    strict_main_checks:false,pr_required:false,checks:[],reason:"EVIDENCE_UNAVAILABLE"};
  if(!policy || !Array.isArray(checkRuns?.check_runs)) return empty;
  const requireChecks=policy.rules?.find(r=>r.type==="required_status_checks")?.parameters;
  const requirePr=policy.rules?.some(r=>r.type==="pull_request")===true;
  const strict=requireChecks?.strict_required_status_checks_policy===true;
  const active=policy.enforcement==="active";
  // A ruleset ID is not proof that the ruleset actually protects main.
  // Fail closed on glob exclusions (the full GitHub fnmatch contract is not
  // implemented here): never overclaim protection for a complex pattern.
  const refs=policy.conditions?.ref_name;
  const protectsMain=policy.target==="branch" &&
    policy.source_type==="Repository" &&
    policy.source==="suvorovpravda/Pravda-Bot" &&
    Array.isArray(refs?.include) &&
    refs.include.includes("refs/heads/main") &&
    Array.isArray(refs.exclude) &&
    refs.exclude.every(pattern=>typeof pattern==="string" &&
      pattern!=="refs/heads/main" && pattern!=="~ALL" &&
      !pattern.includes("*") && !pattern.includes("?"));
  const required=requireChecks?.required_status_checks;
  if(!Array.isArray(required) || required.length===0) {
    return {...empty,state:"FAIL",policy_active:active,
      strict_main_checks:strict,pr_required:requirePr,reason:"MISSING_REQUIRED_CHECKS"};
  }
  const checks=required.map(item=>{
    const name=item?.context;
    const candidates=checkRuns.check_runs.filter(c=>c.name===name &&
      c.head_sha===sourceSha && Number.isSafeInteger(item?.integration_id) &&
      c.app?.id===item.integration_id)
      .sort((a,b)=>Date.parse(b.started_at||0)-Date.parse(a.started_at||0));
    const latest=candidates[0];
    return {name,state:!latest?"MISSING":
      latest.status!=="completed"?"PENDING":
      latest.conclusion==="success"?"PASS":
      latest.conclusion==="failure"?"FAIL":"INCOMPLETE"};
  });
  const configured=active && strict && requirePr && protectsMain;
  const state=!configured?"FAIL":
    checks.every(c=>c.state==="PASS")?"PASS":
    checks.some(c=>c.state==="FAIL")?"FAIL":"PENDING";
  return {state,source_sha:sourceSha,policy_active:active,
    strict_main_checks:strict,pr_required:requirePr,checks,
    reason:configured?"SHA_SCOPED_REQUIRED_CHECKS":
      !protectsMain?"RULESET_NOT_PROVEN_ON_MAIN":"INACTIVE_OR_WEAK_RULESET"};
}

export function buildSnapshot({guide,runs,prs,issues,sourceSha,governancePolicy=null,
  governanceChecks=null,refreshedAtUtc=new Date().toISOString()}) {
  assert.match(refreshedAtUtc,/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$/,
    "Live refresh timestamp must be a concrete UTC instant");
  const history=parseHistorical(guide);
  assert.ok(Array.isArray(prs) && Array.isArray(issues));
  const statuses = WORKFLOWS.map(w=>({
    id:w.id,name:w.name,kind:w.kind,...selectedRun(runs[w.id]||[],sourceSha,w.kind==="observe")
  }));
  const required=statuses.filter(s=>s.kind==="required");
  const passed=required.filter(s=>s.state==="PASS").length;
  const openPrs=prs.filter(p=>p.state==="open").length;
  // Candidate is informational only. It never changes main-only check counts
  // or production readiness. Publish immutable refs, not PR titles or content.
  const candidates=prs.filter(p=>p.state==="open")
    .filter(p=>Number.isSafeInteger(p.number) && p.number>0
      && /^[0-9a-f]{40}$/.test(p.head?.sha||""))
    .sort((a,b)=>a.number-b.number);
  const activePr=candidates.length===1 ? {
    number:candidates[0].number,
    head_sha:candidates[0].head.sha,
    draft:candidates[0].draft===true,
    evidence_scope:"PR_ONLY_NOT_MAIN"
  } : null;

  const openIssues=issues.filter(i=>i.state==="open" && !i.pull_request).length;
  const lastEvent= statuses.map(s=>s.observedAt).filter(Boolean)
    .sort((a,b)=>Date.parse(b)-Date.parse(a))[0]||null;
  return {
    schema:TAG,source_sha:sourceSha,latest_evidence_at:lastEvent,
    refreshed_at_utc:refreshedAtUtc,
    historical:history,required_checks:{passed,total:required.length},
    p0_governance:evaluateGovernance(governancePolicy,governanceChecks,sourceSha),
    workflows:statuses,open_prs:openPrs,active_pr:activePr,open_issues:openIssues,
    production:"NO-GO",trading_live:"OFF",signed_updater:"OFF",
    security_note:"RustSec CI is blocking but SHA-scoped; signed release and offline VM QA remain unverified",
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
  blocks.push(nodeText(1280,119,"P0 GOVERNANCE "+snapshot.p0_governance.state,11,
    snapshot.p0_governance.state==="PASS"?"#54DFC0":"#FFCD76",650));
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
  if(snapshot.active_pr) {
    blocks.push(nodeText(817,279,
      "#"+snapshot.active_pr.number+" · "+snapshot.active_pr.head_sha.slice(0,10)
      +(snapshot.active_pr.draft?" · DRAFT":" · REVIEW"),13,"#BBC5E8",600));
  }
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
  const evidence=snapshot.latest_evidence_at ? new Date(snapshot.latest_evidence_at).toISOString().replace("T"," ").slice(0,16)+" UTC" : "aucune preuve reçue";
  const sync=new Date(snapshot.refreshed_at_utc).toISOString().replace("T"," ").slice(0,16)+" UTC";
  blocks.push(nodeText(73,448,"SYNC  "+sync+"   ·   LATEST MAIN EVIDENCE  "+evidence+"   ·   MAIN SHA  "+snapshot.source_sha.slice(0,10),14,"#A6B5D5",650));
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

/**
 * Read exactly the status contexts configured in the active ruleset.
 * The whole check-runs collection is polluted by skipped status/live workflow
 * runs: requesting only its first 100 entries creates false MISSING states.
 * A failed or incomplete page must produce UNKNOWN, never a fabricated PASS.
 * fetchPage is injected to unit-test pagination without GitHub credentials.
 */
export async function fetchGovernanceChecks(policy, fetchPage) {
  if(!policy) return null;
  const required=policy.rules?.find(rule=>rule.type==="required_status_checks")
    ?.parameters?.required_status_checks;
  if(!Array.isArray(required)) return {check_runs:[]};
  const names=[...new Set(required.map(item=>item?.context))];
  if(names.length>32 || names.some(name=>typeof name!=="string" ||
    !name.trim() || name.length>200)) throw Error("INVALID_REQUIRED_CHECK_CONTEXTS");
  const collected=[];
  for(const name of names) {
    let expectedTotal=null;
    for(let page=1; page<=10; page++) {
      const response=await fetchPage(name,page);
      const count=response?.total_count;
      const rows=response?.check_runs;
      if(!Number.isSafeInteger(count) || count<0 || count>1000 ||
        !Array.isArray(rows) || rows.length>100 ||
        rows.length!==Math.min(100,Math.max(0,count-(page-1)*100)) ||
        (expectedTotal!==null && expectedTotal!==count)) {
        throw Error("INCOMPLETE_GOVERNANCE_CHECK_EVIDENCE");
      }
      expectedTotal=count;
      collected.push(...rows);
      if(page*100>=count) break;
      if(rows.length!==100 || page===10) throw Error("INCOMPLETE_GOVERNANCE_CHECK_EVIDENCE");
    }
  }
  return {check_runs:collected};
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
  const policyPromise=api(prefix+"/rulesets/26400957").catch(()=>null);
  const requests=[
    ...WORKFLOWS.map(w=>api(prefix+"/actions/workflows/"+encodeURIComponent(w.id)+"/runs?branch=main&per_page=14")
      .then(x=>[w.id,x.workflow_runs||[]])),
    api(prefix+"/pulls?state=open&per_page=100").then(x=>["prs",x]),
    api(prefix+"/issues?state=open&per_page=100").then(x=>["issues",x]),
    // Public ruleset and Checks API are read-only. If permissions or an API
    // endpoint are unavailable, the published state becomes UNKNOWN, not PASS.
    policyPromise.then(x=>["policy",x]),
    policyPromise.then(policy=>fetchGovernanceChecks(policy,(name,page)=>
      api(prefix+"/commits/"+source.sha+"/check-runs?check_name="+
        encodeURIComponent(name)+"&per_page=100&page="+page)))
      .catch(()=>null).then(x=>["checkRuns",x])
  ];
  const entries=await Promise.all(requests);
  const data=Object.fromEntries(entries);
  return {sourceSha:source.sha,runs:Object.fromEntries(WORKFLOWS.map(w=>[w.id,data[w.id]])),
    prs:data.prs,issues:data.issues,governancePolicy:data.policy,
    governanceChecks:data.checkRuns};
}

function stableJson(value){return JSON.stringify(value,null,2)+"\n"}
function put(path,content) {
  const dir=dirname(path);
  mkdirSync(dir,{recursive:true});
  try {
    if(readFileSync(path,"utf8")===content) return false;
  } catch(error) {
    if(error?.code!=="ENOENT") throw error;
  }
  // Write into a private sibling directory, then atomically replace the
  // destination. Never follow a destination symlink during the write.
  const temporary=mkdtempSync(join(dir,".pravda-live-"));
  try {
    const staged=join(temporary,"status");
    writeFileSync(staged,content,{flag:"wx",mode:0o600});
    renameSync(staged,path);
  } finally {
    rmSync(temporary,{recursive:true,force:true});
  }
  return true;
}
async function runTests(){
  // Validate atomic replacement, idempotence and cleanup with local fixtures.
  const sandbox=mkdtempSync(join(tmpdir(),"pravda-live-status-test-"));
  try {
    const target=join(sandbox,"status.json");
    assert.equal(put(target,"v1"),true);
    assert.equal(put(target,"v1"),false);
    assert.equal(put(target,"v2"),true);
    assert.equal(readFileSync(target,"utf8"),"v2");
    assert.deepEqual(readdirSync(sandbox),["status.json"]);
  } finally {
    rmSync(sandbox,{recursive:true,force:true});
  }
  // Regression for SonarCloud workflow_run trust boundary: a workflow run from
  // a fork whose branch is also "main" must not activate the write-capable job.
  const workflow=readFileSync(".github/workflows/live-docs.yml","utf8");
  const triggers=workflow.split("\npermissions:\n")[0];
  assert.match(triggers,/\n  push:\n    branches: \[main\]/,
    "A protected-main push must refresh the live feed without manual relaunch");
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
  assert.match(updateJob,/^      checks: read$/m,
    "The governance observer requires only read access to check-runs");
  // The protected main branch MUST NOT be a destination for generated status commits.
  // The read-only README/SECURITY embeds display the separately refreshed status branch.
  assert.doesNotMatch(updateJob,/git push origin HEAD:main|git push origin "HEAD:refs\/heads\/main"/,
    "Direct status pushes to protected main violate the mandatory PR ruleset");
  assert.match(updateJob,/branch="status\/live"/,
    "Live evidence must be published only to the non-release status branch");
  assert.match(updateJob,/git push origin "HEAD:refs\/heads\/\$branch"/,
    "The publisher must push the dedicated status ref");
  assert.match(updateJob,/s\.production!=="NO-GO"/,
    "Publication must explicitly refuse a forged production GO status");
  const remoteImage="https://raw.githubusercontent.com/suvorovpravda/Pravda-Bot/status/live/docs/assets/pravda-live-status.svg";
  const remoteJson="https://github.com/suvorovpravda/Pravda-Bot/blob/status/live/docs/assets/pravda-live-status.json";
  const sonarBadge="https://sonarcloud.io/api/project_badges/measure?project=suvorovpravda_Pravda-Bot&metric=security_rating";
  for(const doc of ["README.md","SECURITY.md"]) {
    const markdown=readFileSync(doc,"utf8");
    assert.ok(markdown.includes(remoteImage),doc+" must render the updated status image");
    assert.ok(markdown.includes(remoteJson),doc+" must link the published JSON");
    assert.ok(markdown.includes(sonarBadge),doc+" must display up-to-date global security rating");
    assert.ok(!markdown.includes("](docs/assets/pravda-live-status.svg)"),
      doc+" must not display the never-updated legacy main-branch image");
  }
  const guide=readFileSync(guidePath,"utf8"), h=parseHistorical(guide);
  assert.equal(h.rows.length,9);
  const sampleRun={head_branch:"main",head_sha:"a".repeat(40),status:"completed",conclusion:"success",
    created_at:"2026-11-06T21:00:00Z",updated_at:"2026-11-06T21:02:00Z"};
  const model=buildSnapshot({guide,sourceSha:"a".repeat(40),
    runs:Object.fromEntries(WORKFLOWS.map(w=>[w.id,[sampleRun]])),
    prs:[{state:"open"}],issues:[{state:"open"}, {state:"open",pull_request:{url:"x"}}]});
  assert.equal(model.required_checks.passed,4);
  assert.equal(model.p0_governance.state,"UNKNOWN");
  const requiredRules={enforcement:"active",target:"branch",
    source_type:"Repository",source:"suvorovpravda/Pravda-Bot",
    conditions:{ref_name:{include:["refs/heads/main"],exclude:[]}},
    rules:[
    {type:"pull_request",parameters:{required_approving_review_count:0}},
    {type:"required_status_checks",parameters:{
      strict_required_status_checks_policy:true,
      required_status_checks:[{context:"CodeQL",integration_id:57789},{context:"Rust check (windows-latest)",integration_id:15368},
        {context:"Windows desktop build and startup",integration_id:15368}]
    }}
  ]};
  const checksOk={check_runs:requiredRules.rules[1].parameters.required_status_checks
    .map(x=>({name:x.context,app:{id:x.integration_id},started_at:"2026-11-10T01:00:00Z",head_sha:"a".repeat(40),
      status:"completed",conclusion:"success"}))};
  // Regression: 204 skipped cockpit check-runs precede the actual contexts.
  // Query required names, not the unfiltered page that returns those skips.
  const requestedNames=[];
  const scoped=await fetchGovernanceChecks(requiredRules,async(name,page)=>{
    requestedNames.push([name,page]);
    const match=checksOk.check_runs.find(row=>row.name===name);
    return {total_count:1,check_runs:[match]};
  });
  assert.deepEqual(requestedNames.map(([name])=>name),
    requiredRules.rules[1].parameters.required_status_checks.map(item=>item.context));
  assert.equal(requestedNames.length,3);
  assert.equal(evaluateGovernance(requiredRules,scoped,"a".repeat(40)).state,"PASS");
  // More than 100 runs for ONE required context must also be paginated.
  const firstHundred=Array.from({length:100},()=>({...checksOk.check_runs[0],
    started_at:"2026-11-09T01:00:00Z",conclusion:"failure"}));
  const pageCalls=[];
  const multiPage=await fetchGovernanceChecks({...requiredRules,rules:[
    requiredRules.rules[0],
    {...requiredRules.rules[1],parameters:{
      ...requiredRules.rules[1].parameters,
      required_status_checks:[requiredRules.rules[1].parameters.required_status_checks[0]]
    }}
  ]},async(name,page)=>{
    pageCalls.push(page);
    return {total_count:101,check_runs:page===1?firstHundred:[checksOk.check_runs[0]]};
  });
  assert.deepEqual(pageCalls,[1,2]);
  assert.equal(multiPage.check_runs.length,101);
  await assert.rejects(()=>fetchGovernanceChecks(requiredRules,async(name,page)=>
    name==="CodeQL"?{total_count:101,check_runs:firstHundred}:{
      total_count:1,check_runs:checksOk.check_runs.filter(row=>row.name===name)
    }),/INCOMPLETE_GOVERNANCE_CHECK_EVIDENCE/);
  const governance=evaluateGovernance(requiredRules,checksOk,"a".repeat(40));
  assert.equal(governance.state,"PASS");
  assert.equal(governance.checks.length,3);
  // Regression for the Code Review: an active ruleset on another branch
  // cannot certify main, even if check runs on main happened to succeed.
  assert.equal(evaluateGovernance({...requiredRules,conditions:{
    ref_name:{include:["refs/heads/develop"],exclude:["refs/heads/main"]}
  }},checksOk,"a".repeat(40)).state,"FAIL");
  assert.equal(evaluateGovernance({...requiredRules,conditions:{
    ref_name:{include:["refs/heads/main"],exclude:["refs/heads/main"]}
  }},checksOk,"a".repeat(40)).state,"FAIL");
  assert.equal(evaluateGovernance({...requiredRules,conditions:{
    ref_name:{include:["refs/heads/main"],exclude:["refs/heads/*"]}
  }},checksOk,"a".repeat(40)).state,"FAIL");
  assert.equal(evaluateGovernance({...requiredRules,target:"tag"},checksOk,
    "a".repeat(40)).state,"FAIL");
  assert.equal(evaluateGovernance({...requiredRules,
    source:"another-owner/another-repository"},checksOk,
    "a".repeat(40)).state,"FAIL");
  assert.equal(evaluateGovernance(requiredRules,checksOk,"b".repeat(40)).state,"PENDING");
  assert.equal(evaluateGovernance(null,checksOk,"a".repeat(40)).state,"UNKNOWN");
  assert.equal(evaluateGovernance(requiredRules,{check_runs:checksOk.check_runs.map(
    x=>({...x,app:{id:999999}}))},"a".repeat(40)).state,"PENDING");
  assert.equal(evaluateGovernance(requiredRules,{check_runs:[
    {...checksOk.check_runs[0],started_at:"2026-11-10T02:00:00Z",conclusion:"failure"},
    ...checksOk.check_runs
  ]},"a".repeat(40)).state,"FAIL");
  assert.equal(evaluateGovernance({...requiredRules,enforcement:"disabled"},checksOk,
    "a".repeat(40)).state,"FAIL");
  assert.equal(evaluateGovernance({...requiredRules,rules:[
    requiredRules.rules[0],
    {...requiredRules.rules[1],parameters:{
      ...requiredRules.rules[1].parameters,strict_required_status_checks_policy:false
    }}]},checksOk,"a".repeat(40)).state,"FAIL");
  assert.equal(evaluateGovernance(requiredRules,{check_runs:checksOk.check_runs.map(
    x=>({...x,conclusion:"failure"}))},"a".repeat(40)).state,"FAIL");
  assert.equal(model.workflows[3].state,"PASS");
  assert.equal(model.workflows[3].auditMode,undefined);
  assert.equal(model.open_issues,1);
  assert.match(model.security_note,/RustSec CI is blocking but SHA-scoped/);
  assert.match(model.security_note,/offline VM QA remain unverified/);
  // One current PR shows its exact candidate commit, never as a main PASS.
  const candidateModel=buildSnapshot({guide,sourceSha:"a".repeat(40),
    runs:Object.fromEntries(WORKFLOWS.map(w=>[w.id,[sampleRun]])),
    prs:[{state:"open",number:172,head:{sha:"b".repeat(40)},draft:true}],
    issues:[]});
  assert.equal(candidateModel.required_checks.passed,4);
  assert.deepEqual(candidateModel.active_pr,{
    number:172,head_sha:"b".repeat(40),draft:true,evidence_scope:"PR_ONLY_NOT_MAIN"
  });
  assert.match(renderSvg(candidateModel),/#172 · bbbbbbbbbb · DRAFT/);
  assert.equal(model.active_pr,null);

  assert.equal(selectedRun([sampleRun],"b".repeat(40)).state,"STALE_SHA");
  assert.equal(selectedRun([],"a".repeat(40)).state,"NOT_RUN");
  const svg=renderSvg(model);
  assert.match(svg,/NO-GO/);
  assert.match(svg,/SYNC  /);
  assert.match(svg,/LATEST MAIN EVIDENCE/);
  assert.match(model.refreshed_at_utc,/Z$/);
  assert.doesNotMatch(svg,/OBSERVE ONLY/);
  assert.match(svg,/RUSTSEC \/ AUDIT BLOQUANT/);
  const failedRustSec={...sampleRun,conclusion:"failure"};
  const failedModel=buildSnapshot({guide,sourceSha:"a".repeat(40),
    runs:Object.fromEntries(WORKFLOWS.map(w=>[w.id,[w.id==="rustsec-audit.yml"?failedRustSec:sampleRun]])),
    prs:[],issues:[]});
  assert.equal(failedModel.required_checks.total,4);
  assert.equal(failedModel.required_checks.passed,3);
  assert.equal(failedModel.workflows[3].state,"FAIL");
  assert.match(renderSvg(failedModel),/FAIL/);
  assert.ok(svg.endsWith("</svg>\n"));
  console.log("PASS: live board percentages, SHA equality, SHA-scoped RustSec, redaction and SVG layout");
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
