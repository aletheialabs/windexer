import { createUmi } from '@metaplex-foundation/umi-bundle-defaults';
import {
  generateSigner,
  signerIdentity,
  publicKey,
  createSignerFromKeypair,
  AccountMeta,
} from '@metaplex-foundation/umi';
import {
  mintFromCandyMachineV2,
  mplCandyMachine,
} from '@metaplex-foundation/mpl-candy-machine';
import {
  TPayloadOutput,
  TPayloadInput,
  deserializeUint8Array,
  calculateRole,
  TPayloadOutputInstruction,
  deserializePrivateKey,
} from '@cambrianone/commons';
import {
  Address,
  createSolanaRpc,
  getAddressFromPublicKey,
} from '@solana/web3.js';
// import { Connection, PublicKey } from '@solana/web3.js';

export default async ({
  executorPDA,
  apiUrl,
  extraSigners,
}: TPayloadInput): Promise<TPayloadOutput> => {
  // Initialize Umi
  const umi = createUmi(apiUrl);
  const proposalSigner = generateSigner(umi);
  umi.use(signerIdentity(proposalSigner)).use(mplCandyMachine());
  // @ts-ignore
  proposalSigner.publicKey = publicKey(executorPDA);

  const nftMintRaw = deserializeUint8Array(extraSigners[0]);
  const nftMintAddress = await getAddressFromPublicKey(
    (await deserializePrivateKey(nftMintRaw)).publicKey,
  );

  const nftOwner = '5N2vEEX7Kseh9paVQjH5fdr69LvHyN44jFjwrTCoo5Ge' as Address;

  const rpc = createSolanaRpc(apiUrl);

  console.log(
    'NFT owner:',
    nftOwner,
    'balance:',
    await rpc.getBalance(nftOwner),
  );

  const nftSigner = createSignerFromKeypair(umi, {
    publicKey: publicKey(nftMintAddress),
    secretKey: Buffer.from(nftMintRaw),
  });

  // Use Umi to create the mint instruction
  const mintInstruction = mintFromCandyMachineV2(umi, {
    nftMint: nftSigner.publicKey,

    candyMachine: publicKey('6XC4ddxeSH8SV1exDKvyAtDVgHKfx4GjkS1DHsviYx4A'),
    collectionMint: publicKey('A6bJCE2g7XKveZxYg4xTEtCiqgewrxYYqazyqCbJV88P'),
    collectionUpdateAuthority: publicKey(
      'GWk2DoJez1mwMWStprqusThgbzW8RmvgznnnXXqWHJoo',
    ),
    nftOwner: publicKey(nftOwner),
    mintAuthority: proposalSigner,
  }).items[0].instruction;

  // Adjust the instruction keys
  const accounts = mintInstruction.keys.map((meta: AccountMeta) => {
    let isSigner = meta.isSigner,
      isWritable = meta.isWritable,
      address = meta.pubkey.toString();
    if (address === umi.payer.publicKey.toString()) {
      address = executorPDA;
      isSigner = false;
      isWritable = true;
    }

    if (address === nftMintAddress.toString()) {
      isSigner = true;
    }

    return {
      role: calculateRole({ isSigner, isWritable }),
      address: address as Address,
    };
  });

  // Prepare any additional instructions if needed
  const proposalInstructions: TPayloadOutputInstruction[] = [
    {
      accounts,
      programAddress: mintInstruction.programId.toString() as Address,
      data: mintInstruction.data,
    },
  ];

  return {
    proposalInstructions,
  };
};
