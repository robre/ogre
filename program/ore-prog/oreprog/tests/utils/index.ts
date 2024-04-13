
import * as anchor from "@coral-xyz/anchor";
import {web3, BN} from "@coral-xyz/anchor";

const PROGRAM_ID = new web3.PublicKey("omcpZynsRS1Py8TP28zeTemamQoRPpuqwdqV8WXnL4M");
const ORE_PROGRAM_ID = new web3.PublicKey("mineRHF5r6S7HyD9SppBfVMXMavDkJsxwGesEvxZr2A");
// const ORE_PROGRAM_ID = new web3.PublicKey("");
// const ORE_TREASURY = new web3.PublicKey("");
// const ORE_BUS = new web3.PublicKey("");

export const findMinerPda = async (initializer: web3.PublicKey, id: number) =>  {
     return web3.PublicKey.findProgramAddressSync(
        [
            anchor.utils.bytes.utf8.encode("x"),
            initializer.toBuffer(),
            [id & 0xff],
        ],
        PROGRAM_ID
    );
    //return (publicKey, bump);
}

export const findProofPda = async (initializer: web3.PublicKey) => {
    return web3.PublicKey.findProgramAddressSync(
        [
            anchor.utils.bytes.utf8.encode("proof"),
            initializer.toBuffer(),
        ],
        ORE_PROGRAM_ID
    );
    return [publicKey, bump];
}
