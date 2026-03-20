// This code only supports `require` with built-in modules; not from file-system.
{% for module in builtinModules -%}
import __require_{{ loop.index }} from '{{ module }}'
{% endfor %}

globalThis.require = function require(filepath) {
  {% for module in builtinModules -%}
  if (filepath === '{{ module }}') return __require_{{ loop.index }};
  {% endfor %}
}

globalThis.createRequire = function createRequire(filepath) {
  return require
}