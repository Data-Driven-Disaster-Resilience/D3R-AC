module.exports = {
  contracts_directory: './contracts',
  contracts_build_directory: './build',
  compilers: {
    solc: {
      version: '0.8.20',
      settings: {
        optimizer: { enabled: true, runs: 200 },
      },
    },
  },
};
