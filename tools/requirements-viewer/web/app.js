'use strict';
const $ = (selector) => document.querySelector(selector);
let graph, selected = null;
const cards = new Map();
function element(tag, text, className) {
  const el = document.createElement(tag);
  if (text !== undefined) el.textContent = text;
  if (className) el.className = className;
  return el;
}
function closure(key, direction) {
  const result = new Set([key]), queue = [key];
  while (queue.length) {
    const current = queue.shift();
    for (const edge of graph.edges) {
      const from = direction === 'up' ? edge.child : edge.parent;
      const to = direction === 'up' ? edge.parent : edge.child;
      if (from === current && !result.has(to)) { result.add(to); queue.push(to); }
    }
  }
  return result;
}
function traceKeys() {
  return selected === null ? new Set() : new Set([...closure(selected, 'up'), ...closure(selected, 'down')]);
}
function select(key) { selected = key; render(); inspect(); }
function render() {
  const query = $('#search').value.toLowerCase().trim(), trace = traceKeys();
  const nodes = graph.nodes.filter(node => (!query || JSON.stringify(node.data).toLowerCase().includes(query)) && ($('#scope').value !== 'trace' || selected === null || trace.has(node.key)));
  const groups = new Map();
  for (const node of nodes) {
    const key = node.layer === null ? 'unresolved' : node.layer;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(node);
  }
  cards.clear(); $('#layers').replaceChildren();
  for (const [layer, entries] of [...groups].sort(([a], [b]) => a === 'unresolved' ? 1 : b === 'unresolved' ? -1 : a - b)) {
    const column = element('section', undefined, 'layer');
    const title = element('h3', layer === 'unresolved' ? 'Unresolved layer' : `Layer ${String(layer).padStart(2, '0')}`);
    title.append(element('span', String(entries.length))); column.append(title);
    for (const node of entries) {
      const card = element('button', undefined, `card${selected === node.key ? ' selected' : trace.has(node.key) ? ' related' : ''}`);
      card.type = 'button'; card.setAttribute('aria-pressed', String(selected === node.key));
      card.append(element('span', node.data.id, 'identifier'), element('strong', node.data.title), element('small', node.documentTitle));
      if (node.data.status !== undefined) card.append(element('span', String(node.data.status), 'badge'));
      card.addEventListener('click', () => select(node.key)); column.append(card); cards.set(node.key, card);
    }
    $('#layers').append(column);
  }
  $('#stats').textContent = `${nodes.length} / ${graph.nodes.length} requirements · ${graph.edges.length} trace links · ${groups.size} visible layers`;
  $('#empty').hidden = nodes.length !== 0;
  requestAnimationFrame(drawEdges);
}
function drawEdges() {
  const svg = $('#edges'), canvas = $('#canvas').getBoundingClientRect();
  svg.replaceChildren();
  for (const edge of graph.edges) {
    const parent = cards.get(edge.parent), child = cards.get(edge.child);
    if (!parent || !child) continue;
    const a = parent.getBoundingClientRect(), b = child.getBoundingClientRect();
    const x1 = a.right - canvas.left, y1 = a.top + a.height / 2 - canvas.top;
    const x2 = b.left - canvas.left, y2 = b.top + b.height / 2 - canvas.top;
    const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    path.setAttribute('d', `M ${x1} ${y1} C ${x1 + 35} ${y1}, ${x2 - 35} ${y2}, ${x2} ${y2}`);
    if (selected === edge.parent || selected === edge.child) path.classList.add('active');
    svg.append(path);
  }
}
function inspect() {
  const detail = $('#detail');
  if (selected === null) { detail.replaceChildren(element('h2', 'Choose a requirement'), element('p', 'Select a card to inspect its statement, trace links, and additional data.')); return; }
  const node = graph.nodes.find(item => item.key === selected);
  detail.replaceChildren(element('div', node.data.id, 'detail-id'), element('h2', node.data.title), element('p', node.data.statement));
  detail.append(element('div', `Source: ${node.source}`, 'section-title'));
  for (const direction of ['Parents', 'Children']) {
    detail.append(element('h3', direction, 'section-title'));
    const links = element('div', undefined, 'links');
    const edges = graph.edges.filter(edge => (direction === 'Parents' ? edge.child : edge.parent) === selected);
    for (const edge of edges) {
      const key = direction === 'Parents' ? edge.parent : edge.child;
      const target = graph.nodes.find(item => item.key === key);
      const button = element('button', target.data.id); button.type = 'button';
      button.addEventListener('click', () => { $('#search').value = ''; select(key); }); links.append(button);
    }
    if (!edges.length) links.append(element('small', 'No resolved links'));
    detail.append(links);
  }
  detail.append(element('h3', 'Additional data', 'section-title'));
  const fields = element('dl');
  for (const [key, value] of Object.entries(node.data)) {
    if (['id', 'title', 'statement'].includes(key)) continue;
    const row = element('div', undefined, 'field'), content = element('dd');
    if (typeof value === 'object' && value !== null) content.append(element('pre', JSON.stringify(value, null, 2)));
    else content.textContent = String(value);
    row.append(element('dt', key), content); fields.append(row);
  }
  if (!fields.children.length) fields.append(element('p', 'No additional fields.'));
  detail.append(fields);
}
async function start() {
  try {
    const response = await fetch('/data');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    graph = await response.json();
    if (graph.issues.length) {
      $('#diagnostics').hidden = false;
      $('#issue-count').textContent = `${graph.issues.length} traceability issues — review unresolved links and layers`;
      for (const issue of graph.issues) $('#issues').append(element('li', issue));
    }
    $('#search').addEventListener('input', render);
    $('#scope').addEventListener('change', render);
    $('#clear').addEventListener('click', () => { selected = null; $('#search').value = ''; $('#scope').value = 'all'; render(); inspect(); });
    new ResizeObserver(drawEdges).observe($('#layers'));
    render();
  } catch (error) { $('#stats').textContent = `Unable to load requirements: ${error.message}`; }
}
start();
