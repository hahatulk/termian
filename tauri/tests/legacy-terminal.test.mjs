import { test } from 'node:test';
import assert from 'node:assert/strict';
import {findOriginalTerminalApi,createOriginalTerminalBridge} from '../src/legacy-terminal.js';

function fixture({protocol='local',localTerminalPath=null,terminalOutput=null}={}) {
  const rendered=[];
  let typed, resized, nativeEvent, focused=0;
  const calls=[],actions=[];
  const id='abc-123';
  const api={
    id,
    terminal:{
      cols:80,rows:30,
      write(data){rendered.push(data)},
      onData(cb){typed=cb;return {dispose(){typed=null}}},
      onResize(cb){resized=cb;return {dispose(){resized=null}}},
      focus(){focused++},
    },
  };
  const root = {__reactFiber$unit:{memoizedProps:{},return:{memoizedProps:{terminalApi:api},return:null}},parentElement:null};
  const element = {parentElement:root};
  let mounted=true;
  const injected=[];
  const flowRoot={dataset:{},querySelector(query){assert.equal(query,'.xterm');return mounted?element:null}};
  const doc = {
    querySelectorAll(query){assert.equal(query,'.xterm');return mounted ? [element] : []},
    getElementById(key){return key===id?flowRoot:null},
    createElement(tag){assert.equal(tag,'style');return {remove(){this.removed=true}}},
    head:{appendChild(style){injected.push(style)}},
  };
  const store={
    state:{localTerminalPath:{path:localTerminalPath,customPaths:[]},terminal:{all:{[id]:{id,connectionProtocol:protocol,host:{address:'localhost'},connectionProcess:'connecting',connectionFlow:{steps:[{name:'connection',status:'progress'}],isConnected:false}}}}},
    getState(){return this.state},
    dispatch(a){actions.push(a); if(a.type==='PATCH_TERMINAL')Object.assign(this.state.terminal.all[id],a.payload.changes)},
  };
  const invokeNative=async (cmd,args)=>{ calls.push({cmd,args}); };
  const listenNative=async (name,cb)=>{
    assert.equal(name,'termian-terminal');
    nativeEvent=cb;return ()=>{};
  };
  const manager=createOriginalTerminalBridge({invokeNative,listenNative,store,doc,terminalOutput});
  return {id,api,store,manager,calls,actions,rendered,doc,flowRoot,injected,focused:()=>focused,
    mount(){mounted=true},
    type(data){typed(data)},
    resize(size){resized(size)},
    event(kind,data){nativeEvent({payload:{sessionId:manager.active.get(id)?.nativeSessionId,kind,data}})},
  };
}

test('Original xterm API is resolved through the desktop React renderer',()=>{
  const f=fixture();
  assert.equal(findOriginalTerminalApi(f.id,f.doc),f.api);
  assert.equal(findOriginalTerminalApi('not-a-terminal',f.doc),null);
});

test('Native PTY drives original xterm output, input, resize, Redux status and close',async()=>{
  const f=fixture();
  f.manager.scan();
  await new Promise(r=>setTimeout(r,20));
  const start=f.calls.find(x=>x.cmd==='start_terminal');
  assert.equal(start.args.sessionId,f.manager.active.get(f.id).nativeSessionId);
  assert.deepEqual(start.args.connection,{serial:null,shell:null});
  assert.equal(start.args.size.cols,80);
  f.event('status','connected');
  assert.equal(f.store.state.terminal.all[f.id].connectionEstablished,true);
  assert.equal(f.store.state.terminal.all[f.id].connectionProcess,'succeeded');
  assert.equal(f.store.state.terminal.all[f.id].connectionFlow,undefined,
    'The original connection-flow screen must close before xterm can render');
  // The legacy Electron controller may report failure after Tauri connected.
  f.store.dispatch({type:'PATCH_TERMINAL',payload:{id:f.id,changes:{connectionProcess:'error'}}});
  f.manager.onAction({type:'CONNECTION_FAILED'});
  assert.equal(f.store.state.terminal.all[f.id].connectionProcess,'succeeded');
  f.store.dispatch({type:'PATCH_TERMINAL',payload:{id:f.id,changes:{
    connectionFlow:{steps:[{name:'connection'}]},connectionProcess:'connecting',
  }}});
  f.manager.onAction({type:'REQUEST_NEW_STEP'});
  assert.equal(f.store.state.terminal.all[f.id].connectionFlow,undefined,
    'Legacy steps must not reactivate the spinner after native connect');
  f.event('output',Buffer.from('hello from native PTY').toString('base64').replace(/=+$/,''));
  assert.equal(Buffer.from(f.rendered[0]).toString(),'hello from native PTY');
  f.type('echo hi\r');
  f.resize({cols:90,rows:25});
  await new Promise(r=>setTimeout(r,10));
  assert.ok(f.calls.some(x=>x.cmd==='write_terminal'&&x.args.data==='echo hi\r'));
  assert.ok(f.calls.some(x=>x.cmd==='resize_terminal'&&x.args.cols===90));
  delete f.store.state.terminal.all[f.id];
  f.manager.scan();
  await new Promise(r=>setTimeout(r,10));
  assert.ok(f.calls.some(x=>x.cmd==='stop_terminal'));
  await f.manager.dispose();
});


test('Local terminal starts the shell configured in Settings',async()=>{
  const f=fixture({localTerminalPath:'C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe'});
  f.manager.scan();
  await new Promise(r=>setTimeout(r,20));
  const start=f.calls.find(x=>x.cmd==='start_terminal');
  assert.equal(start.args.connection.shell,'C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe');
  await f.manager.dispose();
});

test('SSH tabs are left to the existing native connection flow',async()=>{
  const f=fixture({protocol:'ssh'});
  f.manager.scan();
  await new Promise(r=>setTimeout(r,20));
  assert.deepEqual(f.calls,[]);
  await f.manager.dispose();
});

test('Local terminal output is written to its session log and finished on close',async()=>{
  const writers=[];
  class Writer {
    constructor(options){this.options=options;this.chunks=[];this.ended=false;writers.push(this)}
    append(data){this.chunks.push(Buffer.from(data))}
    end(){this.ended=true;this.options.finishedCallback()}
  }
  const f=fixture({terminalOutput:{Writer}});
  f.manager.scan();
  await new Promise(r=>setTimeout(r,20));
  f.event('status','connected');
  // Output before the history entry announces the log is kept for it.
  f.event('output',Buffer.from('prompt$ ').toString('base64'));
  assert.equal(writers.length,0);
  const key=Buffer.alloc(32,7).toString('base64');
  f.manager.onAction({type:'PRE_CREATE_HISTORY',meta:{scope:'local'},payload:{
    terminalId:f.id,connection:{protocol:'local',sessionLogData:{name:'conn-1.log',secretKey:key}},
  }});
  assert.equal(writers.length,1);
  assert.equal(writers[0].options.filename,'conn-1.log');
  assert.deepEqual([...writers[0].options.encryptionKey],[...Buffer.alloc(32,7)]);
  f.event('output',Buffer.from('ls\r\n').toString('base64'));
  assert.equal(Buffer.concat(writers[0].chunks).toString(),'prompt$ ls\r\n');
  delete f.store.state.terminal.all[f.id];
  f.manager.scan();
  assert.equal(writers[0].ended,true);
  assert.ok(f.actions.some(a=>a.type==='FINISH_SESSION_LOG_WRITING'&&a.payload===f.id));
  await f.manager.dispose();
});
