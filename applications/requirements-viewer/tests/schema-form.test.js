const {test} = require('node:test');
const assert = require('node:assert/strict');
const {info, parseValue} = require('../web/schema-form.js');
test('allOf field metadata retains core types and narrows enum choices', () => {
  const metadata = info({allOf: [{type: 'string'}, {enum: ['draft', 'approved']}, {enum: ['approved', 'retired']}]});
  assert.equal(metadata.type, 'string');
  assert.deepEqual(metadata.choices, ['approved']);
});
test('string arrays use line controls and complex objects use JSON values', () => {
  assert.equal(info({allOf: [{type: 'array', items: {type: 'string'}}, {uniqueItems: true}]}).stringArray, true);
  assert.deepEqual(parseValue('lines', 'safety\ncontrols'), ['safety', 'controls']);
  assert.deepEqual(parseValue('json', '{"team":"controls","reviewed":false}'), {team: 'controls', reviewed: false});
  assert.deepEqual(parseValue('lines', ''), []);
});
test('typed values preserve false, zero, null, and reject invalid numeric input', () => {
  assert.equal(parseValue('boolean', 'false'), false);
  assert.equal(parseValue('integer', '0'), 0);
  assert.equal(parseValue('enum', '1', ['low', null]), null);
  assert.throws(() => parseValue('integer', '1.5'));
  assert.throws(() => parseValue('number', ''));
  assert.throws(() => parseValue('json', '{'));
});
