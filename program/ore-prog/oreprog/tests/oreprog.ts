import * as anchor from "@coral-xyz/anchor";
import { Program, BN, web3  } from "@coral-xyz/anchor";
import { publicKey } from "@coral-xyz/anchor/dist/cjs/utils";
import { TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { Oreprog } from "../target/types/oreprog";
import { findMinerPda, findProofPda } from "./utils";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo} from "@solana/spl-token";

const PROGRAM_ID = new web3.PublicKey("omcpZynsRS1Py8TP28zeTemamQoRPpuqwdqV8WXnL4M");
const OMC_TREASURY = new web3.PublicKey("omc1vcb6CmMywXcDxyL77VaPYU98WyyaP3Mx6LBuaTr");
const OMC_ORE_TREASURY = new web3.PublicKey("9idoAEtTrcnoXmrSYMx3pQQYiRLPND3NvcgJnfk6oihW");

const ORE_PROGRAM_ID = new web3.PublicKey("mineRHF5r6S7HyD9SppBfVMXMavDkJsxwGesEvxZr2A");
const ORE_TREASURY = new web3.PublicKey("FTap9fv2GPpWGqrLj3o4c9nHH7p36ih7NbSWHnrkQYqa");
const BUS = new web3.PublicKey("9ShaCzHhQNvH8PLfGyrJbB8MeKHrDnuPMLnUDLJ2yMvz");
const SLOT_HASHES = new anchor.web3.PublicKey("SysvarS1otHashes111111111111111111111111111");

function delay(ms: number) {
    return new Promise( resolve => setTimeout(resolve, ms) );
}

describe("oreprog", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Oreprog as Program<Oreprog>;
  const payer = (program.provider as anchor.AnchorProvider).wallet;
  let connection = anchor.getProvider().connection;

  it("Is initialized!", async () => {
    // Add your test here.
      let miner_id = 0;
      let [minerkey, bump ]: [web3.PublicKey, number] = await findMinerPda(payer.publicKey, miner_id);
      let [proof, bump2] = await findProofPda(minerkey);

      let balance = await anchor.getProvider().connection.getBalance(payer.publicKey);

      // let oreAccount = await getOrCreateAssociatedTokenAccount(connection, payer, )


      console.log(`Miner: ${minerkey} \nProof: ${proof} \nBalance: ${balance/10**9}`);

      const tx = await program.methods.register(miner_id).accounts({
          miner: minerkey,
          proof: proof,
          authority: payer.publicKey,
          minerCollectiveTreasury: OMC_TREASURY,
          ore: ORE_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId
      }).instruction();

      let transaction = new web3.Transaction().add(
          tx
      );
      let hash = await connection.getRecentBlockhash();
      transaction.recentBlockhash = hash.blockhash;
      transaction.feePayer = payer.publicKey;

      console.log(`${transaction}`)

      const signedTx = await payer.signTransaction(transaction);

      let options = {
          maxRetries : 0,
            skipPreflight : true
      };

      let txs = await connection.sendRawTransaction(signedTx.serialize(), options);

      await delay(2000);
      let a = await connection.getParsedTransaction(txs, "confirmed")
      console.log(a.meta.preBalances);
      console.log(a.meta.postBalances);
      console.log(a.meta.err);
      for (let asdf of a. meta.innerInstructions) {
          console.log(`${asdf.index} ${asdf.instructions}`);
          for (let zz of asdf.instructions) {
              console.log(`${zz.programId}`);
          }
      }
      for (let message of a.meta.logMessages) {
          console.log(`${message}`);

      }

      let proof_data = (await connection.getAccountInfo(proof));
      console.log(`proof owner ${proof_data.owner}`);
      console.log(`proof lamports ${proof_data.lamports}`);
      console.log(`proof data ${proof_data.data}`);

      let solutions = [
          {
              id: 0, 
              bump: bump,
              nonce: new BN(2762779), 
              //nonce: new BN(11622396), 
          },
      ];

      const mine_tx = await program.methods.mine(solutions).accounts({
          authority: payer.publicKey,
          bus: BUS,
          treasury: ORE_TREASURY,
          ore: ORE_PROGRAM_ID,
          slotHashes: SLOT_HASHES
      }).remainingAccounts([
          {
              pubkey: minerkey,
              isWritable: true,
              isSigner: false,
          },
          {
              pubkey: proof,
              isWritable: true,
              isSigner: false,
          },
          {
              pubkey: minerkey,
              isWritable: true,
              isSigner: false,
          },
          {
              pubkey: proof,
              isWritable: true,
              isSigner: false,
          }
      ]).instruction();

      let transaction2 = new web3.Transaction().add(
          mine_tx
      );
      let hash2 = await connection.getRecentBlockhash();
      transaction2.recentBlockhash = hash2.blockhash;
      transaction2.feePayer = payer.publicKey;

      //console.log(`${transaction2.}`)

      const signedTx2 = await payer.signTransaction(transaction2);


      let txs2 = await connection.sendRawTransaction(signedTx2.serialize(), options);

      await delay(2000);
      let b = await connection.getParsedTransaction(txs2, "confirmed")
      for (let message of b.meta.logMessages) {
          console.log(`${message}`);

      }

      console.log(`${mine_tx}`);

      // const tx = await program.methods.claim(new BN(1000), miner_id).accounts({
      //     proof: proof,
      //     miner: minerkey,
      //     authority: payer.publicKey,
      //     ore: ORE_PROGRAM_ID,
      //     beneficiary: TODO,
      //     treasury: TODO,
      //     treasuryTokens: TODO,
      //     minerCollectiveOreTreasury: OMC_ORE_TREASURY,
      //     tokenProgram: TOKEN_PROGRAM_ID
      // }).rpc();

      console.log(`Miner: ${minerkey} \nProof: ${proof} \nBalance: ${balance/10**9}`);

    console.log("Your transaction signature", txs);
  });



});
