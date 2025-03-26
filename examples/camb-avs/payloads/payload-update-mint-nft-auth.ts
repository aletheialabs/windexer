import { createUmi } from '@metaplex-foundation/umi-bundle-defaults';
import {
  generateSigner,
  signerIdentity,
  publicKey,
  AccountMeta,
} from '@metaplex-foundation/umi';
import { mplCandyMachine } from '@metaplex-foundation/mpl-candy-machine';
import {
  TPayloadOutput,
  TPayloadInput,
  calculateRole,
} from '@cambrianone/commons';
import { setMintAuthority } from '@metaplex-foundation/mpl-candy-machine/dist/src/generated/instructions/setMintAuthority';
import { address as toAddress } from '@solana/web3.js';

export default async ({
  executorPDA,
  apiUrl,
}: TPayloadInput): Promise<TPayloadOutput> => {
  // Initialize Umi
  const umi = createUmi(apiUrl);
  const proposalSigner = generateSigner(umi);
  umi.use(signerIdentity(proposalSigner)).use(mplCandyMachine());
  // @ts-ignore
  proposalSigner.publicKey = publicKey(executorPDA);

  // Use Umi to create the mint instruction
  const setMintAuth = setMintAuthority(umi, {
    candyMachine: publicKey('6XC4ddxeSH8SV1exDKvyAtDVgHKfx4GjkS1DHsviYx4A'),
    authority: proposalSigner,
    mintAuthority: proposalSigner,
  }).items[0].instruction;

  // Adjust the instruction keys
  const accounts = setMintAuth.keys.map((meta: AccountMeta) => {
    let isSigner = meta.isSigner,
      isWritable = meta.isWritable,
      address = toAddress(meta.pubkey.toString());
    if (meta.pubkey.toString() === umi.payer.publicKey.toString()) {
      address = toAddress(executorPDA);
      isSigner = false;
      isWritable = true;
    }

    return {
      role: calculateRole({ isSigner, isWritable }),
      address,
    };
  });

  accounts.push({
    role: calculateRole({ isSigner: false, isWritable: false }),
    address: toAddress(setMintAuth.programId),
  });

  // Prepare any additional instructions if needed
  const proposalInstructions = [
    {
      accounts,
      programAddress: toAddress(setMintAuth.programId),
      data: setMintAuth.data,
    },
  ];

  return {
    proposalInstructions,
  };
};
