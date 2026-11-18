// AUTO-GENERATED — do not edit. Regenerate with: jsc dom dashboard_component.yaml
// Source: dashboard_component.yaml (version 1)

import { signal, effect } from '@preact/signals-core';

export class DashboardBase extends HTMLElement {
  static readonly tagName = 'js-dashboard';

  // ── Reactive state ─────────────────────────────────────────
  readonly checked = signal(false);
  readonly completed = signal(false);
  readonly disabled = signal(false);
  readonly expanded = signal(false);
  readonly invalid = signal(false);
  readonly loading = signal(false);
  readonly selected = signal(false);

  // ── Effect cleanup handles ──────────────────────────────────────────────
  #cleanups: Array<() => void> = [];

  // ── Element references (private; read-only externally, ADR-0012) ──────────────────────────────────
  // Getter-only stops reassignment (element.button = x); the returned
  // element itself remains a live, mutable DOM node — this does not
  // freeze attributes/children/listeners on what the getter returns.
  #root!: HTMLDivElement;
  get root(): HTMLDivElement { return this.#root; }
  #button!: HTMLButtonElement;
  get button(): HTMLButtonElement {
    if (!this.#button) {
      throw new Error(`[justweb] 'button' accessed before _bindElements() found it - check your markup has the matching [data-part] hook, or that this ran after connectedCallback`);
    }
    return this.#button;
  }
  #input!: HTMLInputElement;
  get input(): HTMLInputElement {
    if (!this.#input) {
      throw new Error(`[justweb] 'input' accessed before _bindElements() found it - check your markup has the matching [data-part] hook, or that this ran after connectedCallback`);
    }
    return this.#input;
  }
  #label!: HTMLSpanElement;
  get label(): HTMLSpanElement {
    if (!this.#label) {
      throw new Error(`[justweb] 'label' accessed before _bindElements() found it - check your markup has the matching [data-part] hook, or that this ran after connectedCallback`);
    }
    return this.#label;
  }
  #link!: HTMLAnchorElement;
  get link(): HTMLAnchorElement {
    if (!this.#link) {
      throw new Error(`[justweb] 'link' accessed before _bindElements() found it - check your markup has the matching [data-part] hook, or that this ran after connectedCallback`);
    }
    return this.#link;
  }

  // ── Slot references (private; read-only externally, ADR-0012) ─────────────────────────────────────
  // Getter-only stops reassignment; the returned element itself
  // remains a live, mutable DOM node.
  #headerSlot!: HTMLDivElement;
  get headerSlot(): HTMLDivElement {
    if (!this.#headerSlot) {
      throw new Error(`[justweb] 'headerSlot' accessed before _bindElements() found it - check your markup has the matching [data-part] hook, or that this ran after connectedCallback`);
    }
    return this.#headerSlot;
  }
  #footerSlot: HTMLDivElement | null = null;
  get footerSlot(): HTMLDivElement | null { return this.#footerSlot; }

  // ── Deferred-bind observer ──────────────────────────
  private _lightDomObserver: MutationObserver | null = null;
  // ── DDAS integrity observer (ADR-0012) ──────────────
  private _ddasIntegrityObserver: MutationObserver | null = null;

  connectedCallback(): void {
    // Element setup
    this.#root = this as unknown as HTMLDivElement;
    this.classList.add('dashboard');

    // Bind light-DOM children + DDAS stamps.
    this._bindElements();
    if (!this._hasAllElements()) {
      this._lightDomObserver = new MutationObserver(() => {
        this._bindElements();
        if (this._hasAllElements() && this._lightDomObserver) {
          this._lightDomObserver.disconnect();
          this._lightDomObserver = null;
          this._watchDdasIntegrity();
        }
      });
      this._lightDomObserver.observe(this, { childList: true, subtree: true });
    } else {
      this._watchDdasIntegrity();
    }

    // Static ARIA
    this.setAttribute('role', 'region');
    this.setAttribute('aria-label', 'Dashboard');

    // Signal effects
    this.#cleanups.push(effect(() => {
      const val = this.checked.value;
      if (val) {
        this.setAttribute('aria-checked', 'true');
      } else {
        this.removeAttribute('aria-checked');
      }
    }));
    this.#cleanups.push(effect(() => {
      const val = this.completed.value;
      if (val) {
        this.setAttribute('data-completed', '');
      } else {
        this.removeAttribute('data-completed');
      }
    }));
    this.#cleanups.push(effect(() => {
      const val = this.disabled.value;
      if (val) {
        this.setAttribute('aria-disabled', 'true');
      } else {
        this.removeAttribute('aria-disabled');
      }
    }));
    this.#cleanups.push(effect(() => {
      const val = this.expanded.value;
      if (val) {
        this.setAttribute('aria-expanded', 'true');
      } else {
        this.removeAttribute('aria-expanded');
      }
    }));
    this.#cleanups.push(effect(() => {
      const val = this.invalid.value;
      if (val) {
        this.setAttribute('aria-invalid', 'true');
      } else {
        this.removeAttribute('aria-invalid');
      }
      if (val) {
        this.setAttribute('data-invalid', '');
      } else {
        this.removeAttribute('data-invalid');
      }
    }));
    this.#cleanups.push(effect(() => {
      const val = this.loading.value;
      if (val) {
        this.setAttribute('aria-busy', 'true');
      } else {
        this.removeAttribute('aria-busy');
      }
    }));
    this.#cleanups.push(effect(() => {
      const val = this.selected.value;
      if (val) {
        this.setAttribute('aria-selected', 'true');
      } else {
        this.removeAttribute('aria-selected');
      }
    }));

    // Event wiring
    this.addEventListener('custom', (e) => {
      this.dispatchEvent(new CustomEvent('increment', { detail: e, bubbles: true, cancelable: false }));
    });
    this.addEventListener('submit', (e) => {
      this.dispatchEvent(new CustomEvent('submit', { detail: e, bubbles: true, cancelable: true }));
    });
  }

  disconnectedCallback(): void {
    this.#cleanups.forEach(fn => fn());
    this.#cleanups = [];
    if (this._lightDomObserver) {
      this._lightDomObserver.disconnect();
      this._lightDomObserver = null;
    }
    if (this._ddasIntegrityObserver) {
      this._ddasIntegrityObserver.disconnect();
      this._ddasIntegrityObserver = null;
    }
  }

  public refresh(): void {
    this._bindElements();
  }

  private _bindElements(): void {
    if (!this.#button) {
      const __el = this.querySelector('[data-part="button"]') as HTMLButtonElement | null;
      if (__el) {
        this.#button = __el;
        __el.setAttribute('data-ddas-id', 'web:dashboard:dashboard:button');
      }
    }
    if (!this.#input) {
      const __el = this.querySelector('[data-part="input"]') as HTMLInputElement | null;
      if (__el) {
        this.#input = __el;
        __el.setAttribute('data-ddas-id', 'web:dashboard:dashboard:input');
      }
    }
    if (!this.#label) {
      const __el = this.querySelector('[data-part="label"]') as HTMLSpanElement | null;
      if (__el) {
        this.#label = __el;
        __el.setAttribute('data-ddas-id', 'web:dashboard:dashboard:label');
      }
    }
    if (!this.#link) {
      const __el = this.querySelector('[data-part="link"]') as HTMLAnchorElement | null;
      if (__el) {
        this.#link = __el;
        __el.setAttribute('data-ddas-id', 'web:dashboard:dashboard:link');
      }
    }
    if (!this.#headerSlot) {
      const __slot = this.querySelector('[slot="header"]') as HTMLDivElement | null;
      if (__slot) this.#headerSlot = __slot;
    }
    if (!this.#footerSlot) {
      this.#footerSlot = this.querySelector('[slot="footer"]') as HTMLDivElement | null;
    }
  }

  private _hasAllElements(): boolean {
    return !!this.#button && !!this.#input && !!this.#label && !!this.#link && !!this.#headerSlot;
  }

  private _watchDdasIntegrity(): void {
    const __strict: boolean = (globalThis as { __justwebRegistryStrict__?: boolean }).__justwebRegistryStrict__ === true;
    const __observer = new MutationObserver((records) => {
      for (const rec of records) {
        if (rec.type !== 'attributes' || rec.attributeName !== 'data-ddas-id') continue;
        const __target = rec.target as Element;
        const __current = __target.getAttribute('data-ddas-id');
        if (__current === rec.oldValue) continue;
        const __msg = `[justweb] data-ddas-id mutated or removed on <${__target.tagName.toLowerCase()}> (was '${rec.oldValue}', now '${__current}')`;
        if (__strict) { throw new Error(__msg); }
        console.warn(__msg);
      }
    });
    __observer.observe(this, { attributes: true, attributeFilter: ['data-ddas-id'], subtree: true, attributeOldValue: true });
    this._ddasIntegrityObserver = __observer;
  }
}

if (typeof customElements === 'object' && customElements !== null && typeof customElements.define === 'function') {
  const __existing = customElements.get('js-dashboard');
  if (!__existing) {
    customElements.define('js-dashboard', DashboardBase);
  } else if (__existing !== DashboardBase) {
    const __msg = "[justweb] custom element 'js-dashboard' is already registered to a different class";
    const __strict: boolean = (globalThis as { __justwebRegistryStrict__?: boolean }).__justwebRegistryStrict__ === true;
    if (__strict) { throw new Error(__msg); }
    console.warn(__msg + '; keeping the existing registration');
  }
}
