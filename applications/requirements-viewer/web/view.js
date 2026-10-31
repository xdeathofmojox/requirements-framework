'use strict';
// Keep helpers private: classic browser scripts share global function bindings.
const RequirementView = (() => {
function closure(graph, key, direction) {
  const result = new Set([key]), queue = [key];
  for (let index = 0; index < queue.length; index++) {
    for (const edge of graph.edges) {
      const from = direction === 'up' ? edge.child : edge.parent;
      const to = direction === 'up' ? edge.parent : edge.child;
      if (from === queue[index] && !result.has(to)) { result.add(to); queue.push(to); }
    }
  }
  return result;
}
function project(graph, mode, anchor) {
  const root = graph.nodes.find(node => node.key === anchor);
  if (!root || mode === 'all') return graph.nodes.map(node => ({...node, viewLayer: node.layer}));
  if (mode === 'parents') {
    const parents = new Set(graph.edges.filter(edge => edge.child === anchor).map(edge => edge.parent));
    return graph.nodes.filter(node => node.key === anchor || parents.has(node.key))
      .map(node => ({...node, viewLayer: node.key === anchor && parents.size ? 1 : 0}));
  }
  if (mode === 'root') {
    const depths = new Map([[anchor, 0]]), queue = [anchor];
    for (let index = 0; index < queue.length; index++) {
      for (const edge of graph.edges) {
        if (edge.parent === queue[index] && !depths.has(edge.child)) {
          depths.set(edge.child, depths.get(edge.parent) + 1); queue.push(edge.child);
        }
      }
    }
    return graph.nodes.filter(node => depths.has(node.key)).map(node => ({...node, viewLayer: depths.get(node.key)}));
  }
  const keys = new Set([...closure(graph, anchor, 'up'), ...closure(graph, anchor, 'down')]);
  return graph.nodes.filter(node => keys.has(node.key)).map(node => ({...node, viewLayer: node.layer}));
}
function connector(a, b, canvas, vertical) {
  if (vertical) {
    const x1 = a.left + a.width / 2 - canvas.left, y1 = a.bottom - canvas.top;
    const x2 = b.left + b.width / 2 - canvas.left, y2 = b.top - canvas.top;
    return `M ${x1} ${y1} C ${x1} ${y1 + 35}, ${x2} ${y2 - 35}, ${x2} ${y2}`;
  }
  const x1 = a.right - canvas.left, y1 = a.top + a.height / 2 - canvas.top;
  const x2 = b.left - canvas.left, y2 = b.top + b.height / 2 - canvas.top;
  return `M ${x1} ${y1} C ${x1 + 35} ${y1}, ${x2 - 35} ${y2}, ${x2} ${y2}`;
}
return {closure, project, connector};
})();
if (typeof module !== 'undefined') module.exports = RequirementView;
