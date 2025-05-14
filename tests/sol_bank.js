const anchor = require("@coral-xyz/anchor");

describe("sol_bank", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  it("Is initialized!", async () => {
    // Add your test here.
    const program = anchor.workspace.solBank;
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
