const main=async()=>{
 const ws=new WebSocket((await (await fetch('http://127.0.0.1:9298/json')).json()).find(p=>p.type==='page').webSocketDebuggerUrl);
 await new Promise((r,j)=>{ws.addEventListener('open',r,{once:true});ws.addEventListener('error',j,{once:true})});
 let seq=0;const pending=new Map(),errors=[];ws.addEventListener('message',e=>{let d=JSON.parse(e.data);if(d.id){pending.get(d.id)?.(d);pending.delete(d.id)}if(d.method==='Runtime.exceptionThrown')errors.push(d.params.exceptionDetails.exception?.description)});
 const call=(m,p={})=>new Promise(r=>{let id=++seq;pending.set(id,r);ws.send(JSON.stringify({id,method:m,params:p}))});
 await call('Runtime.enable');
 const run=async e=>{let v=await call('Runtime.evaluate',{expression:e,returnByValue:true,awaitPromise:true});return v.result.result.value||v.result.result.description};
 const set=async(name,value)=>run('(()=>{let input=document.querySelector(\'input[name="'+name+'"]\');if(!input)return "MISSING";Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value").set.call(input,'+JSON.stringify(value)+');input.dispatchEvent(new Event("input",{bubbles:true}));input.dispatchEvent(new Event("change",{bubbles:true}));return input.value})()');
 console.log('address',await set('address','127.0.0.1'));console.log('label',await set('label','Termian Test Local'));
 console.log('username',await set('username','local-test'));
 await new Promise(r=>setTimeout(r,1000));
 console.log('DOM',JSON.stringify(await run('({text:document.body.innerText.slice(0,1600),buttons:[...document.querySelectorAll("button")].map(e=>({text:e.innerText,aria:e.getAttribute("aria-label"),disabled:e.disabled})).filter(e=>e.text||e.aria),hostForm:document.querySelector("input[name=address]")?.value})')));
 console.log('ERRORS',JSON.stringify(errors.slice(-8)));
 ws.close();
};main().catch(e=>{console.error(e);process.exit(1)});
