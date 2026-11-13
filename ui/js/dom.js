// Tiny element builder. Strings always become text nodes, never HTML: mod names and
// descriptions come from untrusted manifests.
export function h(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [name, value] of Object.entries(attrs)) {
    if (value === false || value == null) continue;
    if (name.startsWith('on')) node.addEventListener(name.slice(2), value);
    else node.setAttribute(name, value === true ? '' : value);
  }
  node.append(...children.flat().filter((c) => c != null && c !== false));
  return node;
}

const SVG = 'http://www.w3.org/2000/svg';

// A symbol from the shared sprite, img/icons.svg.
export function icon(name, className = 'icon') {
  const svg = document.createElementNS(SVG, 'svg');
  svg.setAttribute('class', className);
  svg.setAttribute('aria-hidden', 'true');
  const use = document.createElementNS(SVG, 'use');
  use.setAttribute('href', `img/icons.svg#i-${name}`);
  svg.append(use);
  return svg;
}
