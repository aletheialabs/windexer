import { TPayloadInput, TPayloadOutput } from '@cambrianone/commons';
import { getMintToInstruction } from '@solana-program/token';
import { AccountRole, address } from '@solana/web3.js';

export default async ({
  executorPDA,
}: TPayloadInput): Promise<TPayloadOutput> => {
  const instruction = getMintToInstruction({
    mint: address('CNUv4Cf2P2yTCesXYvF23m2ZmnS9QzKebrJaFh9mY7Su'),
    token: address('DzWFgMFfGQ6Xinsj3SBVQ2sGGm5xRTTJnaDTFT7pmYo6'),
    mintAuthority: address(executorPDA),
    amount: 100,
  });

  instruction.accounts.push({
    role: AccountRole.READONLY,
    address: instruction.programAddress,
  });

  return {
    proposalInstructions: [instruction],
  };
};
