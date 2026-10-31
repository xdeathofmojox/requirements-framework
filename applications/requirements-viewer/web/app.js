'use strict';
const $ = (selector) => document.querySelector(selector);
let graph, selected = null, viewAnchor = null, editingKey = null, pendingDelete = null;
let readSchemaFields = () => ({}), preservedExtra = {};
const cards = new Map();
function element(tag, text, className) {
  const el = document.createElement(tag);
  if (text !== undefined) el.textContent = text;
  if (className) el.className = className;
  return el;
}
function closure(key, direction) { return RequirementView.closure(graph, key, direction); }
function traceKeys() {
  return selected === null ? new Set() : new Set([...closure(selected, 'up'), ...closure(selected, 'down')]);
}
function select(key) { selected = key; if ($('#scope').value !== 'all') viewAnchor = key; render(); inspect(); }
function focusView(mode) {
  if (mode !== 'all' && selected === null) mode = 'all';
  viewAnchor = mode === 'all' ? null : selected;
  $('#scope').value = mode; render();
}
function render() {
  const query = $('#search').value.toLowerCase().trim(), trace = traceKeys();
  const vertical = $('#layout').value === 'vertical';
  $('#canvas').classList.toggle('vertical', vertical);
  const nodes = RequirementView.project(graph, $('#scope').value, viewAnchor)
    .filter(node => !query || JSON.stringify(node.data).toLowerCase().includes(query));
  const groups = new Map();
  for (const node of nodes) {
    const key = node.viewLayer === null ? 'unresolved' : node.viewLayer;
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
      card.append(element('span', node.data.id, 'identifier'), element('strong', node.data.title));
      if (node.data.status !== undefined) card.append(element('span', String(node.data.status), 'badge'));
      card.addEventListener('click', () => select(node.key)); column.append(card); cards.set(node.key, card);
    }
    $('#layers').append(column);
  }
  const visible = new Set(nodes.map(node => node.key));
  const links = graph.edges.filter(edge => visible.has(edge.parent) && visible.has(edge.child)).length;
  $('#stats').textContent = `${nodes.length} / ${graph.nodes.length} requirements · ${links} visible trace links · ${groups.size} visible layers`;
  $('#empty').hidden = nodes.length !== 0;
  $('#empty').textContent = !graph.nodes.length && graph.editable && graph.folders.length
    ? 'No requirements yet. Use Add requirement to start a new document.'
    : 'No requirements match this view.';
  requestAnimationFrame(drawEdges);
}
function drawEdges() {
  const svg = $('#edges'), canvas = $('#canvas').getBoundingClientRect();
  svg.replaceChildren();
  for (const edge of graph.edges) {
    const parent = cards.get(edge.parent), child = cards.get(edge.child);
    if (!parent || !child) continue;
    const a = parent.getBoundingClientRect(), b = child.getBoundingClientRect();
    const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    path.setAttribute('d', RequirementView.connector(a, b, canvas, $('#layout').value === 'vertical'));
    if (selected === edge.parent || selected === edge.child) path.classList.add('active');
    svg.append(path);
  }
}
function inspect() {
  const detail = $('#detail');
  if (selected === null) { detail.replaceChildren(element('h2', 'Choose a requirement'), element('p', 'Select a card to inspect its statement, trace links, and additional data.')); return; }
  const node = graph.nodes.find(item => item.key === selected);
  detail.replaceChildren(element('div', node.data.id, 'detail-id'), element('h2', node.data.title), element('p', node.data.statement));
  if (graph.editable) {
    const edit = element('button', 'Edit requirement'); edit.type = 'button';
    edit.addEventListener('click', () => openEditor(node));
    const remove = element('button', 'Delete requirement', 'danger'); remove.type = 'button';
    remove.addEventListener('click', () => {
      pendingDelete = {key: node.key, revision: graph.revision, confirm_id: node.data.id};
      $('#delete-description').textContent = `${node.data.id} · ${node.data.title}\n${node.source}`;
      const last = graph.nodes.filter(item => item.source === node.source).length === 1;
      $('#delete-title').textContent = last ? 'Delete requirement and file?' : 'Delete requirement?';
      $('#delete-consequence').textContent = last
        ? 'This is the last requirement in the file. Confirming will delete the entire JSON file, including its document metadata. Other loaded requirements must not reference it as a parent.'
        : 'Other loaded requirements must not reference it as a parent, and the remaining document must pass validation.';
      $('#delete-confirm').textContent = last ? 'Yes, delete requirement and file' : 'Yes, delete requirement';
      $('#delete-error').textContent = ''; $('#delete-dialog').showModal();
    });
    const actions = element('div', undefined, 'requirement-actions'); actions.append(edit, remove); detail.append(actions);
  }
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
  const source = element('div', undefined, 'inspector-source');
  source.append(element('span', 'Source', 'eyebrow'), element('p', node.source));
  detail.append(source);
}
function updateDiagnostics() {
  $('#issues').replaceChildren();
  $('#diagnostics').hidden = !graph.issues.length;
  $('#issue-count').textContent = `${graph.issues.length} traceability issues — review unresolved links and layers`;
  for (const issue of graph.issues) $('#issues').append(element('li', issue));
}
function updateTargets() {
  const target = $('#target'), previous = target.selectedOptions[0]?.textContent;
  target.replaceChildren();
  for (const document of graph.documents) {
    const option = element('option', document.path); option.value = document.index;
    option.selected = document.path === previous; target.append(option);
  }
  if (graph.folders.length) target.append(element('option', 'Create a new document…'));
  if (graph.folders.length) target.lastElementChild.value = 'new';
  $('#new-folder').replaceChildren();
  for (const folder of graph.folders) {
    const option = element('option', folder.path); option.value = folder.index; $('#new-folder').append(option);
  }
  updateNewDocument();
  $('#add').disabled = !graph.documents.length && !graph.folders.length;
}
function updateNewDocument() {
  const creating = $('#target').value === 'new';
  $('#new-document').hidden = !creating; $('#new-filename').required = creating;
}
function reloadStatus(message) { $('#reload-status').textContent = message; $('#reload-status').title = message; }
async function reloadFiles() {
  const button = $('#reload');
  button.disabled = true; $('#add').disabled = true;
  reloadStatus('Reloading and validating files…');
  const previous = graph.nodes.find(node => node.key === selected);
  const anchor = graph.nodes.find(node => node.key === viewAnchor);
  try {
    const response = await fetch('/reload', {method: 'POST', headers: {'X-Viewer-Token': graph.token}});
    const body = await response.json();
    if (!response.ok) throw new Error(body.error || `HTTP ${response.status}`);
    graph = body;
    selected = previous ? graph.nodes.find(node => node.source === previous.source && node.data.id === previous.data.id)?.key ?? null : null;
    viewAnchor = anchor ? graph.nodes.find(node => node.source === anchor.source && node.data.id === anchor.data.id)?.key ?? null : null;
    if (viewAnchor === null) $('#scope').value = 'all';
    updateTargets(); updateDiagnostics(); render(); inspect();
    reloadStatus(`Reloaded ${graph.documents.length} files · ${graph.nodes.length} requirements.`);
  } catch (error) { reloadStatus(`Reload failed. Previous snapshot retained. ${error.message}`); }
  finally { button.disabled = false; $('#add').disabled = !graph.documents.length && !graph.folders.length; }
}
function openEditor(node = null) {
  editingKey = node?.key ?? null;
  $('#requirement-form').reset();
  $('#target').disabled = !!node;
  $('#new-id').readOnly = !!node;
  $('#editor-title').textContent = node ? 'Edit requirement' : 'Add requirement';
  $('#editor-help').textContent = node ? 'Update this record in its current file. Its ID stays fixed to preserve trace links.' : 'Append to a JSON file or start a new document.';
  $('#save-error').textContent = '';
  if (node) {
    $('#target').value = graph.documents.find(document => document.path === node.source).index;
    $('#new-id').value = node.data.id; $('#new-title').value = node.data.title;
    $('#new-statement').value = node.data.statement;
    $('#new-parents').value = (node.data.parents || []).join('\n');

  } else if (selected !== null) $('#new-parents').value = graph.nodes.find(node => node.key === selected).data.id;
  const declared = graph.formSchema.fields;
  const core = ['id', 'title', 'statement', 'parents'];
  const fields = Object.fromEntries(Object.entries(declared).filter(([name]) => !core.includes(name)));
  const data = node?.data || {};
  readSchemaFields = SchemaForm.render($('#schema-fields'), fields, data);
  const extra = Object.fromEntries(Object.entries(data).filter(([name]) => !core.includes(name) && !Object.hasOwn(declared, name)));
  preservedExtra = extra;
  $('#new-extra').value = JSON.stringify(extra, null, 2);
  $('#extra-fields-label').hidden = !graph.formSchema.additionalFields;
  updateNewDocument();
  $('#editor').showModal();
}
function setupEditor() {
  $('#mode').textContent = graph.editable ? 'EDITING ENABLED' : 'READ ONLY';
  $('#add').hidden = !graph.editable;
  updateTargets();
  $('#target').addEventListener('change', updateNewDocument);
  $('#add').addEventListener('click', () => openEditor());
  $('#cancel').addEventListener('click', () => $('#editor').close());
  $('#requirement-form').addEventListener('submit', async event => {
    event.preventDefault(); $('#save-error').textContent = ''; $('#save').disabled = true;
    try {
      const extra = graph.formSchema.additionalFields ? JSON.parse($('#new-extra').value) : preservedExtra;
      if (extra === null || Array.isArray(extra) || typeof extra !== 'object') throw new Error('Additional fields must be a JSON object.');
      for (const key of new Set(['id', 'title', 'statement', 'parents', ...Object.keys(graph.formSchema.fields)])) if (Object.hasOwn(extra, key)) throw new Error(`Use the dedicated ${key} field.`);
      const requirement = {...extra, ...readSchemaFields(), id: editingKey !== null ? $('#new-id').value : $('#new-id').value.trim(), title: $('#new-title').value.trim(), statement: $('#new-statement').value.trim(), parents: $('#new-parents').value.split('\n').map(value => value.trim()).filter(Boolean)};
      const creating = editingKey === null && $('#target').value === 'new';
      const payload = editingKey !== null
        ? {key: editingKey, revision: graph.revision, requirement}
        : creating
        ? {folder: Number($('#new-folder').value), filename: $('#new-filename').value.trim(), document: JSON.parse($('#document-fields').value), revision: graph.revision, requirement}
        : {document: Number($('#target').value), revision: graph.revision, requirement};
      const response = await fetch(creating ? '/documents' : '/requirements', {method: editingKey !== null ? 'PUT' : 'POST', headers: {'Content-Type': 'application/json', 'X-Viewer-Token': graph.token}, body: JSON.stringify(payload)});
      const body = await response.text();
      let result;
      try { result = JSON.parse(body); } catch { throw new Error(body || `HTTP ${response.status}`); }
      if (!response.ok) throw new Error(result.error || `HTTP ${response.status}`);
      reloadStatus('');
      graph = result; selected = editingKey ?? graph.nodes.find(node => node.data.id === requirement.id).key;
      $('#search').value = ''; $('#scope').value = 'all'; viewAnchor = null;
      updateTargets(); updateDiagnostics(); render(); inspect(); $('#editor').close();
      $('#requirement-form').reset(); updateNewDocument();
    } catch (error) { $('#save-error').textContent = error.message; }
    finally { $('#save').disabled = false; }
  });
}
function setupDeletion() {
  $('#delete-cancel').addEventListener('click', () => $('#delete-dialog').close());
  $('#delete-dialog').addEventListener('close', () => { pendingDelete = null; });
  $('#delete-dialog').addEventListener('cancel', event => { if ($('#delete-confirm').disabled) event.preventDefault(); });
  $('#delete-form').addEventListener('submit', async event => {
    event.preventDefault();
    if (!pendingDelete) return;
    $('#delete-confirm').disabled = true; $('#delete-cancel').disabled = true;
    $('#delete-error').textContent = '';
    try {
      const response = await fetch('/requirements', {method: 'DELETE', headers: {'Content-Type': 'application/json', 'X-Viewer-Token': graph.token}, body: JSON.stringify(pendingDelete)});
      const body = await response.text();
      let result;
      try { result = JSON.parse(body); } catch { throw new Error(body || `HTTP ${response.status}`); }
      if (!response.ok) throw new Error(result.error || `HTTP ${response.status}`);
      graph = result; selected = null; viewAnchor = null; $('#scope').value = 'all';
      reloadStatus('Requirement deleted.');
      updateTargets(); updateDiagnostics(); render(); inspect(); $('#delete-dialog').close();
    } catch (error) { $('#delete-error').textContent = error.message; }
    finally { $('#delete-confirm').disabled = false; $('#delete-cancel').disabled = false; }
  });
}
async function start() {
  try {
    const response = await fetch('/data');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    graph = await response.json();
    updateDiagnostics(); setupEditor(); setupDeletion();
    $('#reload').addEventListener('click', reloadFiles);
    $('#search').addEventListener('input', render);
    $('#scope').addEventListener('change', () => focusView($('#scope').value));
    $('#layout').addEventListener('change', render);
    $('#clear').addEventListener('click', () => { selected = null; $('#search').value = ''; $('#scope').value = 'all'; viewAnchor = null; render(); inspect(); });
    new ResizeObserver(drawEdges).observe($('#layers'));
    render();
  } catch (error) { $('#stats').textContent = `Unable to load requirements: ${error.message}`; }
}
start();
