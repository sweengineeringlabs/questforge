// dashboard — browser test (auto-generated)
// Source: dashboard_component.yaml (v1)
// Run:    mweb test dashboard_browser_test.gen.ts --browser
//
// DO NOT EDIT — regenerate with: mweb generate component dashboard

import { Browser, launchBrowser } from '@browser/browser';
import { Page } from '@browser/page';
import { defaultLaunchConfig, findChromeBinary } from '@browser/manager';

let _passed: number = 0;
let _failed: number = 0;

function assert(cond: boolean, msg: string): void {
  if (cond) { _passed++; console.log('  PASS:', msg); }
  else { _failed++; console.error('  FAIL:', msg); }
}

let chrome: string = findChromeBinary();
if (chrome === '') { console.log('Chrome not found'); throw new Error('Chrome is required for browser tests'); }

let browser: Browser = launchBrowser(defaultLaunchConfig());
let __err: string = "";
try {
  let page: Page = browser.newPage();
  page.goto('http://localhost:8090');
  page.waitForSelector('div');

  console.log('1. Dashboard root element renders');
  const root = page.querySelector('div');
  assert(root !== null, 'Dashboard root <div> must exist in DOM');

  console.log('2. Dashboard ARIA role is set');
  const ariaRole = page.evaluate("document.querySelector('div')?.getAttribute('role')");
  assert(ariaRole === 'region', 'root must have role="region"');

  console.log('3. Dashboard ARIA label is set');
  const ariaLabel = page.evaluate("document.querySelector('div')?.getAttribute('aria-label')");
  assert(ariaLabel === 'Dashboard', 'root must have aria-label="Dashboard"');

  console.log('4. Dashboard element button is reachable');
  const el_button = page.querySelector('[data-addr="button"]');
  assert(el_button !== null, 'button must have DOM address [data-addr="button"]');

  console.log('5. Dashboard element input is reachable');
  const el_input = page.querySelector('[data-addr="input"]');
  assert(el_input !== null, 'input must have DOM address [data-addr="input"]');

  console.log('6. Dashboard element label is reachable');
  const el_label = page.querySelector('[data-addr="label"]');
  assert(el_label !== null, 'label must have DOM address [data-addr="label"]');

  console.log('7. Dashboard element link is reachable');
  const el_link = page.querySelector('[data-addr="link"]');
  assert(el_link !== null, 'link must have DOM address [data-addr="link"]');

  console.log('8. Dashboard element button is clickable');
  if (el_button !== null) {
   page.click('[data-addr="button"]');
   page.waitForTimeout(50);
   const active_button: string = page.evaluate("document.activeElement?.getAttribute('data-addr') || document.activeElement?.tagName || ''");
   assert(active_button === 'button' || active_button.toLowerCase() === 'button', 'button must receive focus after click, got: ' + active_button);
  }

  console.log('9. Dashboard element input is clickable');
  if (el_input !== null) {
   page.click('[data-addr="input"]');
   page.waitForTimeout(50);
   const active_input: string = page.evaluate("document.activeElement?.getAttribute('data-addr') || document.activeElement?.tagName || ''");
   assert(active_input === 'input' || active_input.toLowerCase() === 'input', 'input must receive focus after click, got: ' + active_input);
  }

  console.log('10. Dashboard event increment (custom) fires');
  page.evaluate("window.__evt_increment = null");
  page.evaluate("document.querySelector('div').addEventListener('increment', function(e) { window.__evt_increment = 'received'; })");
  page.evaluate("document.querySelector('div').dispatchEvent(new CustomEvent('increment'))");
  page.waitForTimeout(50);
  const evt_increment: string = page.evaluate("String(window.__evt_increment)");
  assert(evt_increment === 'received', 'increment event must be received by listener, got: ' + evt_increment);

  console.log('11. Dashboard event submit (submit) fires');
  page.evaluate("window.__evt_submit = null");
  page.evaluate("document.querySelector('div').addEventListener('submit', function(e) { window.__evt_submit = 'received'; })");
  page.evaluate("document.querySelector('div').dispatchEvent(new Event('submit', {bubbles:true}))");
  page.waitForTimeout(50);
  const evt_submit: string = page.evaluate("String(window.__evt_submit)");
  assert(evt_submit === 'received', 'submit event must be received by listener, got: ' + evt_submit);

} catch (e: unknown) {
  __err = String(e);
}
browser.close();
console.log('\n' + _passed + ' passed, ' + _failed + ' failed');
if (__err !== '') { console.error('test threw:', __err); throw new Error('Generated test failed'); }
if (_failed > 0) { throw new Error('Generated test failed'); }
