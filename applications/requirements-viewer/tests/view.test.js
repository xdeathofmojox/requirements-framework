const {test} = require('node:test');
const assert = require('node:assert/strict');
const {project, connector} = require('../web/view.js');
const graph = {
  nodes: ['root', 'parent', 'selected', 'child', 'leaf', 'sibling'].map((key, index) => ({key, layer: [0, 1, 2, 3, 4, 2][index]})),
  edges: [['root', 'parent'], ['parent', 'selected'], ['selected', 'child'], ['child', 'leaf'], ['parent', 'sibling']].map(([parent, child]) => ({parent, child})),
};
test('selected root shows only itself and descendants, rebased to layer zero', () => {
  const nodes = project(graph, 'root', 'selected');
  assert.deepEqual(nodes.map(node => [node.key, node.viewLayer]), [['selected', 0], ['child', 1], ['leaf', 2]]);
  assert.equal(graph.nodes[2].layer, 2);
});
test('one layer up includes only immediate parents and the selection', () => {
  assert.deepEqual(project(graph, 'parents', 'selected').map(node => [node.key, node.viewLayer]), [['parent', 0], ['selected', 1]]);
  assert.deepEqual(project(graph, 'parents', 'root').map(node => node.key), ['root']);
});
test('full view restores all nodes; missing focus falls back to full view', () => {
  assert.equal(project(graph, 'all', 'selected').length, 6);
  assert.deepEqual(project(graph, 'root', 'missing'), project(graph, 'all', null));
});
test('root projection terminates on cycles', () => {
  const cyclic = {...graph, edges: [...graph.edges, {parent: 'leaf', child: 'selected'}]};
  assert.equal(project(cyclic, 'root', 'selected').length, 3);
});
test('connectors attach to horizontal sides or vertical top/bottom', () => {
  const a = {left: 10, right: 110, top: 20, bottom: 70, width: 100, height: 50};
  const b = {left: 200, right: 300, top: 150, bottom: 200, width: 100, height: 50};
  assert.equal(connector(a, b, {left: 0, top: 0}, false), 'M 110 45 C 145 45, 165 175, 200 175');
  assert.equal(connector(a, b, {left: 0, top: 0}, true), 'M 60 70 C 60 105, 250 115, 250 150');
});


test('selected trace includes ancestors and descendants but excludes siblings', () => {
  assert.deepEqual(project(graph, 'trace', 'selected').map(node => node.key), ['root', 'parent', 'selected', 'child', 'leaf']);
});

test('browser scripts do not overwrite the trace traversal helper', () => {
  const vm = require('node:vm');
  const fs = require('node:fs');
  const path = require('node:path');
  // Pause startup before DOM access; load both classic scripts in one global scope.
  const context = vm.createContext({fixture: graph, fetch: () => new Promise(() => {})});
  for (const file of ['view.js', 'app.js']) {
    vm.runInContext(fs.readFileSync(path.join(__dirname, '../web', file), 'utf8'), context);
  }
  const keys = vm.runInContext("graph = fixture; RequirementView.project(graph, 'trace', 'selected').map(node => node.key)", context);
  assert.deepEqual([...keys], ['root', 'parent', 'selected', 'child', 'leaf']);
});
