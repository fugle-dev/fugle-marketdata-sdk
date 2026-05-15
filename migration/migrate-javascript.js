/**
 * jscodeshift transform to migrate v0.2.x code to v0.3.0 API.
 *
 * Transforms:
 * 1. new RestClient('key') -> new RestClient({ apiKey: 'key' })
 * 2. new WebSocketClient('key') -> new WebSocketClient({ apiKey: 'key' })
 * 3. new RestClient(variable) -> new RestClient({ apiKey: variable })
 *
 * Usage:
 *   # Dry run (preview changes):
 *   npx jscodeshift -t migration/migrate-javascript.js src/ --dry
 *
 *   # Apply changes:
 *   npx jscodeshift -t migration/migrate-javascript.js src/
 *
 *   # Process specific file:
 *   npx jscodeshift -t migration/migrate-javascript.js examples/auth.js
 *
 *   # With verbose output:
 *   npx jscodeshift -t migration/migrate-javascript.js src/ --verbose
 *
 * @param {Object} fileInfo - File information including path and source
 * @param {Object} api - jscodeshift API
 * @param {Object} options - Transform options
 * @returns {string|null} - Modified source code or null if unchanged
 */
module.exports = function(fileInfo, api, options) {
  const j = api.jscodeshift;
  const root = j(fileInfo.source);

  let modified = false;

  // Find all NewExpression nodes for RestClient or WebSocketClient
  root.find(j.NewExpression, {
    callee: {
      type: 'Identifier',
      name: name => name === 'RestClient' || name === 'WebSocketClient'
    }
  }).forEach(path => {
    const node = path.node;

    // Only transform if there's exactly 1 argument
    if (node.arguments.length !== 1) {
      return;
    }

    const firstArg = node.arguments[0];

    // Skip if already an ObjectExpression (already migrated)
    if (firstArg.type === 'ObjectExpression') {
      return;
    }

    // Only transform if argument is a Literal (string) or Identifier (variable)
    if (firstArg.type !== 'Literal' && firstArg.type !== 'Identifier') {
      return;
    }

    // Create the new options object: { apiKey: <value> }
    const optionsObject = j.objectExpression([
      j.property(
        'init',
        j.identifier('apiKey'),
        firstArg
      )
    ]);

    // Replace the argument
    node.arguments = [optionsObject];
    modified = true;
  });

  // Return modified source code, or null if no changes made
  return modified ? root.toSource({ quote: 'single' }) : null;
};
