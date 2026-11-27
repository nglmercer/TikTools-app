/** A complete MediaQueryList for stubbing `window.matchMedia`: a real EventTarget with a fixed `matches`. */
class StaticMediaQueryList extends EventTarget implements MediaQueryList {
  onchange: MediaQueryList['onchange'] = null;
  constructor(
    readonly matches: boolean,
    readonly media: string,
  ) {
    super();
  }
  addListener() {}
  removeListener() {}
}

export function mediaQueryList(matches: boolean, media = ''): MediaQueryList {
  return new StaticMediaQueryList(matches, media);
}
