'use strict';
const SchemaForm = (() => {
  function fragments(schema) {
    if (!schema || typeof schema !== 'object') return [];
    return [schema, ...(schema.allOf || []).flatMap(fragments)];
  }
  function info(schema) {
    const parts = fragments(schema);
    let choices;
    for (const part of parts) {
      const values = Object.hasOwn(part, 'const') ? [part.const] : part.enum;
      if (values) choices = choices ? choices.filter(value => values.some(other => JSON.stringify(value) === JSON.stringify(other))) : values;
    }
    const types = parts.map(part => part.type).filter(Boolean);
    const type = types.find(type => typeof type === 'string');
    const items = parts.map(part => part.items).filter(item => item && typeof item === 'object');
    return {
      choices, type,
      stringArray: type === 'array' && items.some(item => fragments(item).some(part => part.type === 'string')),
      description: parts.find(part => part.description)?.description,
      defaultValue: parts.find(part => Object.hasOwn(part, 'default'))?.default,
      complex: parts.some(part => part.oneOf || part.anyOf || Array.isArray(part.type)),
    };
  }
  function parseValue(kind, value, choices) {
    if (kind === 'enum') return choices[Number(value)];
    if (kind === 'boolean') return value === 'true';
    if (kind === 'number' || kind === 'integer') {
      if (value.trim() === '') throw new Error('Enter a number.');
      const number = Number(value);
      if (!Number.isFinite(number) || (kind === 'integer' && !Number.isInteger(number))) throw new Error(`Enter a valid ${kind}.`);
      return number;
    }
    if (kind === 'lines') return value === '' ? [] : value.split('\n');
    if (kind === 'json') return JSON.parse(value);
    return value;
  }
  function render(container, fields, data) {
    container.replaceChildren();
    const readers = [];
    for (const [name, field] of Object.entries(fields)) {
      if (field.schema === false) continue;
      const meta = info(field.schema), row = document.createElement('fieldset');
      row.className = 'schema-field';
      const legend = document.createElement('legend'); legend.textContent = name + (field.required ? ' *' : ''); row.append(legend);
      const value = Object.hasOwn(data, name) ? data[name] : meta.defaultValue;
      const kind = meta.choices ? 'enum' : meta.complex ? 'json' : meta.stringArray ? 'lines' : ['string','number','integer','boolean'].includes(meta.type) ? meta.type : 'json';
      const input = document.createElement(kind === 'enum' || kind === 'boolean' ? 'select' : kind === 'json' || kind === 'lines' ? 'textarea' : 'input');
      input.setAttribute('aria-label', name);
      if (kind === 'enum' || kind === 'boolean') {
        const choices = kind === 'enum' ? meta.choices : [false, true];
        choices.forEach((choice, index) => {
          const option = document.createElement('option'); option.value = kind === 'enum' ? String(index) : String(choice);
          option.textContent = typeof choice === 'string' ? choice : JSON.stringify(choice);
          option.selected = JSON.stringify(choice) === JSON.stringify(value); input.append(option);
        });
      } else {
        if (kind === 'number' || kind === 'integer') { input.type = 'number'; input.step = kind === 'integer' ? '1' : 'any'; }
        if (kind === 'lines') input.value = Array.isArray(value) ? value.join('\n') : '';
        else if (kind === 'json') { input.rows = 3; input.value = value === undefined ? '' : JSON.stringify(value, null, 2); }
        else input.value = value === undefined ? '' : String(value);
      }
      let include;
      if (!field.required) {
        const label = document.createElement('label'); label.className = 'include-field';
        include = document.createElement('input'); include.type = 'checkbox'; include.checked = value !== undefined;
        label.append(include, document.createTextNode('Include field')); row.append(label);
        input.disabled = !include.checked;
        include.addEventListener('change', () => { input.disabled = !include.checked; });
      }
      input.required = !!field.required && kind !== 'lines'; row.append(input);
      const hint = document.createElement('small');
      hint.textContent = [meta.description, kind === 'json' ? 'JSON value' : kind === 'lines' ? 'One value per line' : ''].filter(Boolean).join(' · ');
      if (hint.textContent) row.append(hint);
      container.append(row);
      readers.push(() => {
        if (include && !include.checked) return null;
        try { return [name, parseValue(kind, input.value, meta.choices)]; }
        catch (error) { throw new Error(`${name}: ${error.message}`); }
      });
    }
    return () => Object.fromEntries(readers.map(read => read()).filter(Boolean));
  }
  return {info, parseValue, render};
})();
if (typeof module !== 'undefined') module.exports = SchemaForm;
