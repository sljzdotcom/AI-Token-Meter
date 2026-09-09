const terminalProtocolResponse = /\x1b(?:\[1;1R|\[\?[0-9]+(?:;[0-9]+)*c)/g;

function normalizeTerminalInput(input) {
 return input.replace(terminalProtocolResponse, '');
}

module.exports = { normalizeTerminalInput };
