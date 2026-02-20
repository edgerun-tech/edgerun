import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";

describe("edgerun", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const deployedProgramId = new PublicKey(
    "AgjxA2CoMmmWXrcsJtvvpmqdRHLVHrhYf6DAuBCL4s5T"
  );

  it("loads deployed localnet program", async () => {
    const info = await provider.connection.getAccountInfo(deployedProgramId);
    expect(info).to.not.equal(null);
    expect(info!.executable).to.equal(true);
  });
});
