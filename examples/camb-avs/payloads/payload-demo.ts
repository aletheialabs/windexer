import { TPayloadInput, TPayloadOutput } from '@cambrianone/commons';
import { address, createNoopSigner, lamports } from '@solana/web3.js';
import { getTransferSolInstruction } from '@solana-program/system';

export default async ({
  executorPDA,
}: TPayloadInput): Promise<TPayloadOutput> => {
  try {
    const instruction = getTransferSolInstruction({
      source: createNoopSigner(address(executorPDA)),
      destination: address('DnXet6kPAWkk2bjC55wvqKkRKkLcMAvdGxeAniNyM2GY'),
      amount: lamports(5000000n),
    });

    return {
      proposalInstructions: [{ ...instruction }],
    };
  } catch (e) {
    console.error('Payload error', e);
    throw e;
  }
};
