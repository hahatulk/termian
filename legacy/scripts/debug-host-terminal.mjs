const main=async()=>{
 const pages=await(await fetch('http://127.0.0.1:9298/json')).json();
 const ws=new WebSocket(pages.find(p=>p.type==='page').webSocketDebuggerUrl);
 await new Promise((rs,rj)=>{ws.addEventListener('open',rs,{once:true});ws.addEventListener('error',rj,{once:true})});
 let id=0;const pending=new Map();const messages=[];
 ws.addEventListener('message',e=>{const v=JSON.parse(e.data);if(v.id){pending.get(v.id)?.(v);pending.delete(v.id);}else if(v.method==='Runtime.exceptionThrown')messages.push('EXCEPTION: '+(v.params.exceptionDetails.exception?.description||v.params.exceptionDetails.text));else if(v.method==='Runtime.consoleAPICalled'&&v.params.type==='error')messages.push('CONSOLE: '+v.params.args.map(x=>x.value||x.description).join(' '));});
 const call=(method,params={})=>new Promise(r=>{const i=++id;pending.set(i,r);ws.send(JSON.stringify({id:i,method,params}))});
 await call('Runtime.enable');
 const run=async expr=>{const r=await call('Runtime.evaluate',{expression:expr,returnByValue:true,awaitPromise:true});if(r.result.exceptionDetails)console.log('ERROR',r.result.result.description);return r.result.result.value};
 const dom=()=>run('({text:document.body.innerText.slice(0,2200),buttons:[...document.querySelectorAll("button")].map(e=>({text:e.innerText,aria:e.getAttribute("aria-label"),title:e.title,disabled:e.disabled})).slice(0,60),inputs:[...document.querySelectorAll("input")].map(e=>({placeholder:e.placeholder,value:e.value,outer:e.outerHTML.slice(0,200)})).slice(0,25)})');
 console.log('BEFORE',JSON.stringify(await dom()).slice(0,9000));
 await run('(()=>{let e=[...document.querySelectorAll("button")].find(x=>x.innerText.trim()==="NEW HOST");if(e){e.click();return true}return false})()');
 await new Promise(r=>setTimeout(r,1100));
 console.log('AFTER_NEW_HOST',JSON.stringify(await dom()).slice(0,11000));
 console.log('ERRORS',JSON.stringify(messages.slice(-7)));
 ws.close();
};
main().catch(e=>{console.error(e);process.exit(1)});
