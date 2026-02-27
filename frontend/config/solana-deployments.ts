// SPDX-License-Identifier: Apache-2.0
export const solanaDeploymentsConfig = {
  programs: {
    edgerunProgram: {
      label: 'Edgerun Program',
      programIdByCluster: {
        localnet: 'A2ac8yDnTXKfZCHWqcJVYFfR2jv65kezW95XTgrrdbtG',
        devnet: 'A2ac8yDnTXKfZCHWqcJVYFfR2jv65kezW95XTgrrdbtG',
        testnet: '',
        'mainnet-beta': ''
      }
    }
  }
} as const
