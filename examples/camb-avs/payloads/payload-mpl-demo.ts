import {
  deserializePrivateKey,
  TPayloadInput,
  TPayloadOutput,
} from '@cambrianone/commons';

import {
  AuthorityType,
  getSetAuthorityInstruction,
} from '@solana-program/token';
import { address, createSignerFromKeyPair } from '@solana/web3.js';

export default async ({
  executorPDA,
  extraSigners,
}: TPayloadInput): Promise<TPayloadOutput> => {
  const [nftMintSigner, userSigner] = await Promise.all(
    extraSigners.map(
      async (ser) =>
        await createSignerFromKeyPair(await deserializePrivateKey(ser)),
    ),
  );

  return {
    proposalInstructions: [
      getSetAuthorityInstruction({
        owned: nftMintSigner.address,
        owner: userSigner,
        authorityType: AuthorityType.MintTokens,
        newAuthority: address(executorPDA),
      }),
    ],
  };
};
