const target=process.env.CDP_URL||'http://127.0.0.1:9298/json';
const pages=await(await fetch(target)).json();
const ws=new WebSocket(pages.find(p=>p.type==='page').webSocketDebuggerUrl);
await new Promise((r,j)=>{ws.addEventListener('open',r,{once:true});ws.addEventListener('error',j,{once:true})});
const expression=process.argv.slice(2).join(' ');
ws.addEventListener('message',e=>{const d=JSON.parse(e.data);if(d.id===1){console.log(JSON.stringify(d.result?.result?.value??d.result?.result?.description??d.result,null,2).slice(0,60000));ws.close()}});
ws.send(JSON.stringify({id:1,method:'Runtime.evaluate',params:{expression,returnByValue:true,awaitPromise:true}}));
