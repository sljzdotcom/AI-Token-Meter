// Synthetic no-network CLI process for Windows ConPTY lifecycle integration tests.
const fs=require('node:fs');
const path=require('node:path');
const {normalizeTerminalInput}=require('./gemini-terminal-input.cjs');
const scenario=process.env.AI_METER_TEST_SCENARIO;
if (fs.readFileSync('.env','utf8')!=='' || process.env.NO_BROWSER!=='true' || process.env.GEMINI_CLI_TRUST_WORKSPACE!=='false') process.exit(7);
const policy=JSON.parse(fs.readFileSync(process.env.GEMINI_CLI_SYSTEM_SETTINGS_PATH,'utf8'));
if(policy.hooksConfig.enabled!==false)process.exit(8);
const stages=[];
const recordStage=stage=>{
 if(!stages.includes(stage))stages.push(stage);
 fs.writeFileSync(process.env.AI_METER_TEST_STAGE,stages.join('>'));
};
recordStage('started');
process.stdin.setRawMode(true);
fs.writeFileSync(process.env.AI_METER_TEST_PID,String(process.pid));
const root=process.env.AI_METER_TEST_FIXTURES;
let rawInput='';
let input='';
let modelOpened=false;
let modelClosed=false;
let quitSeen=false;
process.stdout.write(fs.readFileSync(path.join(root,'untrusted-ready.ansi.txt')));
recordStage('ready-written');
process.stdin.on('data',chunk=>{
 rawInput+=chunk.toString();fs.writeFileSync(process.env.AI_METER_TEST_RAW_INPUT,rawInput);
 input=normalizeTerminalInput(rawInput);fs.writeFileSync(process.env.AI_METER_TEST_INPUT,input);
 recordStage('input-received');
 if(!modelOpened && input==='/model\r' && scenario!=='hang') {
  modelOpened=true;
  process.stdout.write('\x1b[2J\x1b[H'+fs.readFileSync(path.join(root,'authenticated-untrusted-model-open.ansi.txt'),'utf8'));
  recordStage('model-opened');
 }
 if(modelOpened && !modelClosed && input==='/model\r\x1b') {
  modelClosed=true;
  process.stdout.write('\x1b[2J\x1b[H'+fs.readFileSync(path.join(root,'untrusted-ready.ansi.txt'),'utf8'));
  recordStage('model-closed');
 }
 if(modelClosed && !quitSeen && input==='/model\r\x1b/quit\r') {
  quitSeen=true;
  recordStage('quit-received');
  if(scenario==='exit-conflict') process.stdout.write('\x1b[2J\x1b[H'+fs.readFileSync(path.join(root,'authenticated-untrusted-model-open.ansi.txt'),'utf8').replaceAll('25%','30%'),()=>process.exit(0));
  else process.exit(0);
 }
});
setInterval(()=>{},1000);
