const solc = require('solc');
const fs = require('fs');
const path = require('path');

function readSource(p) {
  return fs.readFileSync(p, 'utf8');
}

const input = {
  language: 'Solidity',
  sources: {
    'D3RACToken.sol': { content: readSource('contracts/D3RACToken.sol') },
    'IdentityRegistry.sol': { content: readSource('contracts/IdentityRegistry.sol') },
    'DisbursementController.sol': { content: readSource('contracts/DisbursementController.sol') },
    'RiskRegistry.sol': { content: readSource('contracts/RiskRegistry.sol') },
    'FundingRequestRegistry.sol': { content: readSource('contracts/FundingRequestRegistry.sol') },
    'AttestorMultisig.sol': { content: readSource('contracts/AttestorMultisig.sol') }
  },
  settings: {
    outputSelection: {
      '*': { '*': ['abi', 'evm.bytecode.object'] }
    },
    optimizer: { enabled: true, runs: 200 }
  }
};

const output = JSON.parse(solc.compile(JSON.stringify(input)));

let hasError = false;
if (output.errors) {
  for (const err of output.errors) {
    if (err.severity === 'error') hasError = true;
    console.log(`[${err.severity}] ${err.formattedMessage}`);
  }
}
if (hasError) process.exit(1);

fs.mkdirSync('artifacts-raw', { recursive: true });

for (const [file, contracts] of Object.entries(output.contracts)) {
  for (const [name, contract] of Object.entries(contracts)) {
    const out = {
      abi: contract.abi,
      bytecode: '0x' + contract.evm.bytecode.object
    };
    fs.writeFileSync(path.join('artifacts-raw', `${name}.json`), JSON.stringify(out, null, 2));
    console.log(`Compiled ${name} -> artifacts-raw/${name}.json (${out.bytecode.length / 2 - 1} bytes)`);
  }
}
