
globalThis.queueMicrotask = function queueMicrotask(callback) { Promise.resolve().then(callback); };
globalThis.TextEncoder = class TextEncoder {
  get encoding() { return "utf-8"; }
  encode(input = "") {
    const s = String(input), out = [];
    for (let i = 0; i < s.length; i++) {
      let c = s.charCodeAt(i);
      if (c >= 0xd800 && c <= 0xdbff && i + 1 < s.length) {
        const d = s.charCodeAt(i + 1);
        if (d >= 0xdc00 && d <= 0xdfff) { c = 0x10000 + ((c - 0xd800) << 10) + (d - 0xdc00); i++; }
      }
      if (c >= 0xd800 && c <= 0xdfff) c = 0xfffd;
      if (c < 0x80) out.push(c);
      else if (c < 0x800) out.push(0xc0 | (c >> 6), 0x80 | (c & 63));
      else if (c < 0x10000) out.push(0xe0 | (c >> 12), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
      else out.push(0xf0 | (c >> 18), 0x80 | ((c >> 12) & 63), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
    }
    return Uint8Array.from(out);
  }
};
globalThis.TextDecoder = class TextDecoder {
  get encoding() { return "utf-8"; }
  decode(input) {
    if (input === undefined) return "";
    const b = input instanceof Uint8Array ? input
      : ArrayBuffer.isView(input) ? new Uint8Array(input.buffer, input.byteOffset, input.byteLength)
      : new Uint8Array(input);
    let s = "";
    for (let i = 0; i < b.length;) {
      const c = b[i];
      let n = c < 0x80 ? 0 : c >= 0xf0 && c < 0xf5 ? 3 : c >= 0xe0 ? 2 : c >= 0xc2 && c < 0xe0 ? 1 : -1;
      let cp = n === 0 ? c : n === 3 ? c & 7 : n === 2 ? c & 15 : c & 31;
      if (n < 0 || i + n >= b.length) { s += "\ufffd"; i++; continue; }
      let ok = true;
      for (let k = 1; k <= n; k++) { const x = b[i + k]; if ((x & 0xc0) !== 0x80) { ok = false; break; } cp = (cp << 6) | (x & 63); }
      if (!ok) { s += "\ufffd"; i++; continue; }
      s += String.fromCodePoint(cp > 0x10ffff ? 0xfffd : cp);
      i += n + 1;
    }
    return s;
  }
};

// ---- URLSearchParams / URL --------------------------------------------
const enc = (s) => encodeURIComponent(s).replace(/%20/g, "+").replace(/[!'()~]/g, (c) => "%" + c.charCodeAt(0).toString(16).toUpperCase());
const dec = (s) => { try { return decodeURIComponent(s.replace(/\+/g, " ")); } catch { return s; } };
globalThis.URLSearchParams = class URLSearchParams {
  #list = [];
  #url = null;
  constructor(init = "") {
    if (typeof init === "string") {
      for (const part of init.replace(/^\?/, "").split("&")) {
        if (!part) continue;
        const k = part.indexOf("=");
        this.#list.push(k < 0 ? [dec(part), ""] : [dec(part.slice(0, k)), dec(part.slice(k + 1))]);
      }
    } else if (init && typeof init[Symbol.iterator] === "function") {
      for (const [k, v] of init) this.#list.push([String(k), String(v)]);
    } else if (init && typeof init === "object") {
      for (const k of Object.keys(init)) this.#list.push([k, String(init[k])]);
    }
  }
  _attach(url) { this.#url = url; }
  #touch() { if (this.#url) this.#url._setSearch(this.toString()); }
  append(k, v) { this.#list.push([String(k), String(v)]); this.#touch(); }
  delete(k) { this.#list = this.#list.filter(([n]) => n !== String(k)); this.#touch(); }
  get(k) { const e = this.#list.find(([n]) => n === String(k)); return e ? e[1] : null; }
  getAll(k) { return this.#list.filter(([n]) => n === String(k)).map(([, v]) => v); }
  has(k) { return this.#list.some(([n]) => n === String(k)); }
  set(k, v) {
    k = String(k); v = String(v);
    const i = this.#list.findIndex(([n]) => n === k);
    if (i < 0) this.#list.push([k, v]);
    else { this.#list[i][1] = v; this.#list = this.#list.filter(([n], j) => n !== k || j === i); }
    this.#touch();
  }
  sort() { this.#list.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0)); this.#touch(); }
  forEach(cb, thisArg) { for (const [k, v] of this.#list) cb.call(thisArg, v, k, this); }
  keys() { return this.#list.map(([k]) => k)[Symbol.iterator](); }
  values() { return this.#list.map(([, v]) => v)[Symbol.iterator](); }
  entries() { return this.#list.map(([k, v]) => [k, v])[Symbol.iterator](); }
  [Symbol.iterator]() { return this.entries(); }
  get size() { return this.#list.length; }
  toString() { return this.#list.map(([k, v]) => enc(k) + "=" + enc(v)).join("&"); }
};
const DEFAULT_PORTS = { "http:": "80", "https:": "443", "ws:": "80", "wss:": "443", "ftp:": "21" };
globalThis.URL = class URL {
  #p = {};
  #params;
  constructor(input, base) {
    input = String(input).trim();
    let m = /^([a-zA-Z][a-zA-Z0-9+.-]*):(.*)$/.exec(input);
    if (!m) {
      if (base === undefined) throw new TypeError("Invalid URL: " + input);
      const b = new URL(base);
      if (input.startsWith("//")) input = b.protocol + input;
      else if (input.startsWith("/")) input = b.origin + input;
      else if (input.startsWith("?")) input = b.origin + b.pathname + input;
      else if (input.startsWith("#")) input = b.origin + b.pathname + b.search + input;
      else input = b.origin + b.pathname.replace(/[^/]*$/, "") + input;
      m = /^([a-zA-Z][a-zA-Z0-9+.-]*):(.*)$/.exec(input);
    }
    const protocol = m[1].toLowerCase() + ":";
    let rest = m[2];
    let hash = "", search = "";
    let i = rest.indexOf("#"); if (i >= 0) { hash = rest.slice(i); rest = rest.slice(0, i); }
    i = rest.indexOf("?"); if (i >= 0) { search = rest.slice(i); rest = rest.slice(0, i); }
    let username = "", password = "", hostname = "", port = "", pathname = rest;
    if (rest.startsWith("//")) {
      rest = rest.slice(2);
      const slash = rest.indexOf("/");
      let auth = slash < 0 ? rest : rest.slice(0, slash);
      pathname = slash < 0 ? "/" : rest.slice(slash);
      const at = auth.lastIndexOf("@");
      if (at >= 0) {
        const cred = auth.slice(0, at); auth = auth.slice(at + 1);
        const c = cred.indexOf(":");
        username = c < 0 ? cred : cred.slice(0, c); password = c < 0 ? "" : cred.slice(c + 1);
      }
      const pm = /^(.*?)(?::(\d*))?$/.exec(auth);
      hostname = pm[1].toLowerCase(); port = pm[2] || "";
      if (port === DEFAULT_PORTS[protocol]) port = "";
    }
    if (search === "?") search = "";
    if (hash === "#") hash = "";
    const segs = [];
    for (const s of pathname.split("/").slice(1)) { if (s === "..") segs.pop(); else if (s !== ".") segs.push(s); }
    if (rest !== undefined && pathname.startsWith("/")) pathname = "/" + segs.join("/");
    pathname = pathname.replace(/ /g, "%20");
    this.#p = { protocol, username, password, hostname, port, pathname, search, hash };
    this.#params = new URLSearchParams(search);
    this.#params._attach(this);
  }
  _setSearch(text) { this.#p.search = text ? "?" + text : ""; }
  get protocol() { return this.#p.protocol; } set protocol(v) { this.#p.protocol = String(v).replace(/:?$/, ":"); }
  get username() { return this.#p.username; } set username(v) { this.#p.username = String(v); }
  get password() { return this.#p.password; } set password(v) { this.#p.password = String(v); }
  get hostname() { return this.#p.hostname; } set hostname(v) { this.#p.hostname = String(v).toLowerCase(); }
  get port() { return this.#p.port; } set port(v) { v = String(v); this.#p.port = v === DEFAULT_PORTS[this.#p.protocol] ? "" : v; }
  get host() { return this.#p.hostname + (this.#p.port ? ":" + this.#p.port : ""); }
  set host(v) { const m = /^(.*?)(?::(\d*))?$/.exec(String(v)); this.hostname = m[1]; this.port = m[2] || ""; }
  get pathname() { return this.#p.pathname; } set pathname(v) { v = String(v); this.#p.pathname = v.startsWith("/") ? v : "/" + v; }
  get search() { return this.#p.search; }
  set search(v) { v = String(v); this.#p.search = v && v !== "?" ? (v.startsWith("?") ? v : "?" + v) : ""; this.#params = new URLSearchParams(this.#p.search); this.#params._attach(this); }
  get hash() { return this.#p.hash; } set hash(v) { v = String(v); this.#p.hash = v && v !== "#" ? (v.startsWith("#") ? v : "#" + v) : ""; }
  get searchParams() { return this.#params; }
  get origin() { return this.#p.hostname ? this.#p.protocol + "//" + this.host : "null"; }
  get href() {
    const p = this.#p;
    const auth = p.username ? p.username + (p.password ? ":" + p.password : "") + "@" : "";
    return p.protocol + (p.hostname || p.protocol === "file:" ? "//" + auth + this.host : "") + p.pathname + p.search + p.hash;
  }
  toString() { return this.href; }
  toJSON() { return this.href; }
  static canParse(input, base) { try { new URL(input, base); return true; } catch { return false; } }
};

// ---- Buffer (a Uint8Array with Node-style helpers) --------------------
const B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
function bytesFrom(value, encoding) {
  if (typeof value === "string") {
    encoding = String(encoding || "utf8").toLowerCase();
    if (encoding === "hex") {
      const out = new Uint8Array(value.length >> 1);
      for (let i = 0; i < out.length; i++) { const b = parseInt(value.substr(i * 2, 2), 16); if (Number.isNaN(b)) return out.subarray(0, i); out[i] = b; }
      return out;
    }
    if (encoding === "base64" || encoding === "base64url") {
      const clean = value.replace(/-/g, "+").replace(/_/g, "/").replace(/[^A-Za-z0-9+/]/g, "");
      const out = [];
      for (let i = 0; i < clean.length; i += 4) {
        const n = [0, 1, 2, 3].map((k) => (i + k < clean.length ? B64.indexOf(clean[i + k]) : -1));
        out.push((n[0] << 2) | (n[1] >> 4));
        if (n[2] >= 0) out.push(((n[1] & 15) << 4) | (n[2] >> 2));
        if (n[3] >= 0) out.push(((n[2] & 3) << 6) | n[3]);
      }
      return Uint8Array.from(out);
    }
    if (encoding === "latin1" || encoding === "binary" || encoding === "ascii") return Uint8Array.from(value, (c) => c.charCodeAt(0) & 255);
    return new TextEncoder().encode(value);
  }
  if (value instanceof ArrayBuffer) return new Uint8Array(value);
  if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
  return Uint8Array.from(value);
}
globalThis.Buffer = class Buffer extends Uint8Array {
  static from(value, encoding) {
    const src = bytesFrom(value, encoding);
    const out = new Buffer(src.length);
    out.set(src);
    return out;
  }
  static alloc(size, fill = 0) { const b = new Buffer(size); if (fill) b.fill(typeof fill === "string" ? fill.charCodeAt(0) : fill); return b; }
  static allocUnsafe(size) { return new Buffer(size); }
  static isBuffer(value) { return value instanceof Buffer; }
  static byteLength(value, encoding) { return typeof value === "string" ? bytesFrom(value, encoding).length : value.byteLength; }
  static concat(list, total) {
    total ??= list.reduce((n, b) => n + b.length, 0);
    const out = Buffer.alloc(total);
    let at = 0;
    for (const b of list) { if (at >= total) break; out.set(b.subarray(0, total - at), at); at += b.length; }
    return out;
  }
  toString(encoding = "utf8", start = 0, end = this.length) {
    const view = this.subarray(start, end);
    encoding = String(encoding).toLowerCase();
    if (encoding === "hex") return Array.from(view, (b) => b.toString(16).padStart(2, "0")).join("");
    if (encoding === "base64" || encoding === "base64url") {
      let out = "";
      for (let i = 0; i < view.length; i += 3) {
        const [a, b, c] = [view[i], view[i + 1], view[i + 2]];
        out += B64[a >> 2] + B64[((a & 3) << 4) | ((b ?? 0) >> 4)];
        out += b === undefined ? "=" : B64[((b & 15) << 2) | ((c ?? 0) >> 6)];
        out += c === undefined ? "=" : B64[c & 63];
      }
      return encoding === "base64url" ? out.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "") : out;
    }
    if (encoding === "latin1" || encoding === "binary" || encoding === "ascii") return String.fromCharCode(...view);
    return new TextDecoder().decode(view);
  }
  toJSON() { return { type: "Buffer", data: Array.from(this) }; }
  equals(other) { return this.length === other.length && this.every((b, i) => b === other[i]); }
  slice(start, end) { return this.subarray(start, end); }
  readUInt8(o = 0) { return this[o]; }
  readUInt16LE(o = 0) { return this[o] | (this[o + 1] << 8); }
  readUInt32LE(o = 0) { return (this[o] | (this[o + 1] << 8) | (this[o + 2] << 16) | (this[o + 3] << 24)) >>> 0; }
  writeUInt8(v, o = 0) { this[o] = v; return o + 1; }
  writeUInt16LE(v, o = 0) { this[o] = v; this[o + 1] = v >> 8; return o + 2; }
  writeUInt32LE(v, o = 0) { this[o] = v; this[o + 1] = v >> 8; this[o + 2] = v >> 16; this[o + 3] = v >> 24; return o + 4; }
};
