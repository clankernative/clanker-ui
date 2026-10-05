// Run through ego-browser nodejs after admitting tests/fixtures/static-native-proof.html
// in a real Native app. Requires an authenticated TaskSpace, not a file:// preview.
const fs = await import('node:fs/promises');
const { default: assert } = await import('node:assert/strict');
// Prepend globalThis.staticNativeProof = {origin, space, screenshots} to the input.
// ego-browser intentionally does not forward arbitrary shell environment variables.
const {origin, space, screenshots} = globalThis.staticNativeProof ?? {};
assert.ok(origin && space && screenshots, 'provide staticNativeProof origin, space and screenshots');
const task = await taskSpace(space);
const page = task.page('p1');
await page.goto(origin + '/layouts#static-port-proof');
await page.waitForSelector('#static-port-proof');
await page.cdp('Emulation.setDeviceMetricsOverride', {width:1440,height:1800,deviceScaleFactor:1,mobile:false});
await page.goto(origin + '/layouts#static-port-proof');
const semantics = await page.evaluate(() => {
  const root = document.querySelector('#static-port-proof');
  const component = name => [...root.querySelectorAll(`[data-cui-component="${name}"]`)];
  const feed = component('activity-feed')[0];
  const tabs = component('tabs')[0];
  const segmented = component('segmented-control')[0];
  const steps = component('progress-steps');
  return {
    feed: {count:feed.querySelectorAll('ol[aria-label] > li').length, times:[...feed.querySelectorAll('time')].map(n=>n.dateTime)},
    definitions:component('definition-list').map(n=>({tag:n.tagName,terms:n.querySelectorAll('dt').length,values:n.querySelectorAll('dd').length})),
    disclosures:component('disclosure').map(n=>({tag:n.tagName,summary:n.firstElementChild.tagName,open:n.open})),
    groups:component('button-group').map(n=>({role:n.getAttribute('role'),label:n.getAttribute('aria-label')})),
    buttons:[...root.querySelectorAll('button')].map(n=>({disabled:n.disabled,busy:n.getAttribute('aria-busy')})),
    tabs:{tag:tabs.tagName,label:tabs.getAttribute('aria-label'),links:tabs.querySelectorAll('a').length,current:tabs.querySelectorAll('[aria-current="page"]').length},
    segmented:{tag:segmented.tagName,links:segmented.querySelectorAll('a').length,current:segmented.querySelector('[aria-current="page"]').tagName,disabled:segmented.querySelector('[aria-disabled="true"]').tagName,disabledTabIndex:segmented.querySelector('[aria-disabled="true"]').tabIndex},
    steps:steps.map(n=>({tag:n.tagName,label:n.getAttribute('aria-label'),list:n.querySelector('ol').tagName,current:n.querySelectorAll('[aria-current="step"]').length,links:n.querySelectorAll('a').length,text:n.textContent.replace(/\s+/g,' ')})),
    fakeTabs:root.querySelectorAll('[role="tablist"],[role="tab"],[role="tabpanel"]').length,
    hrefs:[...root.querySelectorAll('a')].map(n=>n.pathname),
    scripts:[...document.scripts].map(n=>n.src),
    commands:root.querySelectorAll('form,[data-on\\:click],[data-on\\:submit]').length,
    unresolved:root.querySelectorAll('cui-slot,cui-tabs,cui-disclosure').length,
    overflow:document.documentElement.scrollWidth>innerWidth,
  };
});
assert.deepEqual(semantics.feed,{count:2,times:['2026-09-30T09:00:00Z','2026-10-01T09:00:00Z']});
assert.deepEqual(semantics.definitions,[{tag:'DL',terms:3,values:3},{tag:'DL',terms:1,values:1}]);
assert.deepEqual(semantics.disclosures,[{tag:'DETAILS',summary:'SUMMARY',open:false},{tag:'DETAILS',summary:'SUMMARY',open:true}]);
assert.ok(semantics.groups.every(n=>n.role==='group'&&n.label));
assert.ok(semantics.buttons.length===2&&semantics.buttons.every(n=>n.disabled));
assert.equal(semantics.buttons[1].busy,'true');
assert.deepEqual(semantics.tabs,{tag:'NAV',label:'Gallery sections',links:3,current:1});
assert.deepEqual(semantics.segmented,{tag:'NAV',links:1,current:'SPAN',disabled:'SPAN',disabledTabIndex:-1});
assert.ok(semantics.steps.every(n=>n.tag==='NAV'&&n.label&&n.list==='OL'&&n.current===1));
assert.deepEqual(semantics.steps.map(n=>n.links),[1,0]);
assert.ok(semantics.steps[0].text.includes('Completed:')&&semantics.steps[1].text.includes('Blocked:'));
assert.ok(semantics.hrefs.every(n=>['/tasks','/layouts','/components'].includes(n)));
assert.ok(semantics.scripts.every(n=>!/(activity-feed|tabs|segmented-control|progress-steps|disclosure|button-group|definition-list)/.test(n)));
assert.equal(semantics.fakeTabs,0);assert.equal(semantics.commands,0);assert.equal(semantics.unresolved,0);assert.equal(semantics.overflow,false);

const contrast = await page.evaluate(()=>{
  const rgb=s=>s.match(/[\d.]+/g).map(Number);
  const lum=c=>c.slice(0,3).map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4}).reduce((n,v,i)=>n+v*[.2126,.7152,.0722][i],0);
  return [...document.querySelectorAll('#proof-tabs a,#proof-segmented-control a')].map(n=>{
    let p=n,bg;while(p){bg=rgb(getComputedStyle(p).backgroundColor);if(bg.length===3||bg[3]===1)break;p=p.parentElement}
    const text=lum(rgb(getComputedStyle(n).color)),back=lum(bg);
    return {label:n.textContent.trim(),ratio:(Math.max(text,back)+.05)/(Math.min(text,back)+.05)};
  });
});
assert.ok(contrast.every(n=>n.ratio>=4.5),'app-owned navigation colors must remain readable');
const summary='#proof-disclosure details:first-of-type > summary';
await page.focus(summary); await page.press(summary,'Space');
assert.equal(await page.evaluate(()=>document.querySelector('#proof-disclosure details').open),true);
await page.press(summary,'Enter');
assert.equal(await page.evaluate(()=>document.querySelector('#proof-disclosure details').open),false);
const first='#proof-button-group [aria-label="Gallery destinations"] a:first-child';
await page.focus(first);await page.press(first,'Tab');
assert.equal(await page.evaluate(()=>document.activeElement.textContent.trim()),'Component explorer');
await page.keyboard.press('Tab');
assert.equal(await page.evaluate(()=>document.activeElement.textContent.trim()),'Layout atlas');
await page.focus('#proof-tabs li:first-child a');
await page.keyboard.press('Tab');
await page.keyboard.press('Enter');
await page.waitForURL(origin+'/tasks');
await page.goto(origin+'/layouts#static-port-proof');
await page.waitForSelector('#static-port-proof');
await page.waitForFunction(()=>document.fonts.status==='loaded');
await page.screenshot({path:screenshots+'/gallery-static-native-desktop.png'});

const mobile=await task.newPage();
await mobile.cdp('Emulation.setDeviceMetricsOverride',{width:390,height:844,deviceScaleFactor:1,mobile:true});
await mobile.goto(origin+'/layouts#static-port-proof');await mobile.waitForSelector('#static-port-proof');
const mobileLayout=await mobile.evaluate(()=>({overflow:document.documentElement.scrollWidth>innerWidth,problems:[...document.querySelectorAll('#static-port-proof article')].filter(n=>n.scrollWidth>n.clientWidth||n.getBoundingClientRect().right>innerWidth).map(n=>n.id)}));
assert.equal(mobileLayout.overflow,false);assert.deepEqual(mobileLayout.problems,[]);
await mobile.screenshot({path:screenshots+'/gallery-static-native-mobile.png'});
await mobile.focus('#proof-tabs li:first-child a');
await mobile.screenshot({path:screenshots+'/gallery-static-native-mobile-navigation.png'});
await mobile.focus('#proof-tabs li:last-child a');
assert.equal(await mobile.evaluate(()=>{const r=document.querySelector('#proof-tabs li:last-child a').getBoundingClientRect();return r.left>=0&&r.right<=innerWidth}),true,'keyboard focus reveals horizontally scrolled destinations');

const nojs=await task.newPage();
await nojs.cdp('Emulation.setScriptExecutionDisabled',{value:true});
await nojs.cdp('Emulation.setDeviceMetricsOverride',{width:1440,height:1040,deviceScaleFactor:1,mobile:false});
await nojs.goto(origin+'/layouts#static-port-proof');await nojs.waitForSelector('#static-port-proof');
assert.equal(await nojs.evaluate(()=>document.querySelector('#proof-disclosure details').open),false);
await nojs.focus(summary);await nojs.press(summary,'Space');
assert.equal(await nojs.evaluate(()=>document.querySelector('#proof-disclosure details').open),true);
await nojs.press(summary,'Enter');
assert.equal(await nojs.evaluate(()=>document.querySelector('#proof-disclosure details').open),false);
await nojs.focus('#proof-segmented-control a');await nojs.press('#proof-segmented-control a','Enter');
await nojs.waitForURL(origin+'/tasks');
await nojs.goto(origin+'/layouts#static-port-proof');
await nojs.focus('#proof-progress-steps a');await nojs.press('#proof-progress-steps a','Enter');
await nojs.waitForURL(origin+'/tasks');
console.log(JSON.stringify({passed:true,semantics,mobileLayout,contrast,keyboard:'Disclosure Space/Enter; link Enter; disabled buttons skipped',noJavaScript:'Disclosure and navigation work with script execution disabled',pages:{desktop:page.label,mobile:mobile.label,nojs:nojs.label}},null,2));
