// No imports needed: web3, anchor, pg, and more are globally available
//TODO: review and edit test file 

describe("xLST Index Tests", () => {
  // Generate necessary keypairs
  const adminKp = new web3.Keypair();
  const userKp = new web3.Keypair();
  let indexTokenMint;
  let globalStatePda;
  let userAccount;
  let liquidityPool;
  let userTokenAccount;
  let protocolTokenAccount;

  const TOKEN_PROGRAM_ID = new web3.PublicKey(
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
  ); // Standard Token Program ID

  before(async () => {
    // Airdrop SOL to admin and user
    const airdropAdmin = await pg.connection.requestAirdrop(
      adminKp.publicKey,
      2 * web3.LAMPORTS_PER_SOL
    );
    await pg.connection.confirmTransaction(airdropAdmin);

    const airdropUser = await pg.connection.requestAirdrop(
      userKp.publicKey,
      2 * web3.LAMPORTS_PER_SOL
    );
    await pg.connection.confirmTransaction(airdropUser);

    // Create Mint Account
    indexTokenMint = await createMint(
      pg.connection,
      adminKp,
      adminKp.publicKey,
      null,
      9
    );

    // Create PDAs
    [globalStatePda] = await web3.PublicKey.findProgramAddressSync(
      [Buffer.from("global-state")],
      pg.program.programId
    );

    [userAccount] = await web3.PublicKey.findProgramAddressSync(
      [Buffer.from("user"), userKp.publicKey.toBuffer()],
      pg.program.programId
    );

    [liquidityPool] = await web3.PublicKey.findProgramAddressSync(
      [Buffer.from("liquidity-pool")],
      pg.program.programId
    );

    // Create Token Accounts
    userTokenAccount = await createTokenAccount(
      pg.connection,
      adminKp,
      indexTokenMint,
      userKp.publicKey
    );

    protocolTokenAccount = await createTokenAccount(
      pg.connection,
      adminKp,
      indexTokenMint,
      globalStatePda
    );
  });

  it("Initialize Protocol", async () => {
    const baseYieldRate = new BN(500); // 5% annual yield

    const txHash = await pg.program.methods
      .initialize({
        baseYieldRate: baseYieldRate,
      })
      .accounts({
        globalState: globalStatePda,
        admin: adminKp.publicKey,
        indexTokenMint: indexTokenMint,
        systemProgram: web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([adminKp])
      .rpc();

    await pg.connection.confirmTransaction(txHash);

    const globalState = await pg.program.account.globalState.fetch(globalStatePda);
    assert(globalState.admin.equals(adminKp.publicKey));
    assert(globalState.baseYieldRate.eq(baseYieldRate));
    assert(globalState.indexTokenMint.equals(indexTokenMint));
  });

  it("Mint xLST", async () => {
    const mintAmount = new BN(100_000_000); // 100 tokens with 6 decimals

    const txHash = await pg.program.methods
      .mintXlst(mintAmount)
      .accounts({
        userAccount: userAccount,
        globalState: globalStatePda,
        liquidityPool: liquidityPool,
        userTokenAccount: userTokenAccount,
        protocolTokenAccount: protocolTokenAccount,
        indexTokenMint: indexTokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([userKp])
      .rpc();

    await pg.connection.confirmTransaction(txHash);

    const userAccountInfo = await pg.program.account.userAccount.fetch(userAccount);
    assert(userAccountInfo.balance.eq(mintAmount));

    const tokenBalance = await getTokenBalance(
      pg.connection,
      userTokenAccount
    );
    assert(tokenBalance.eq(mintAmount));
  });

  it("Update Yield Rate", async () => {
    const newYieldRate = new BN(600); // 6% annual yield

    const txHash = await pg.program.methods
      .updateYield(newYieldRate)
      .accounts({
        globalState: globalStatePda,
        admin: adminKp.publicKey,
      })
      .signers([adminKp])
      .rpc();

    await pg.connection.confirmTransaction(txHash);

    const globalState = await pg.program.account.globalState.fetch(globalStatePda);
    assert(globalState.baseYieldRate.eq(newYieldRate));
  });

  it("Burn xLST", async () => {
    const burnAmount = new BN(50_000_000); // 50 tokens

    const txHash = await pg.program.methods
      .burnXlst(burnAmount)
      .accounts({
        userAccount: userAccount,
        globalState: globalStatePda,
        userTokenAccount: userTokenAccount,
        indexTokenMint: indexTokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([userKp])
      .rpc();

    await pg.connection.confirmTransaction(txHash);

    const userAccountInfo = await pg.program.account.userAccount.fetch(userAccount);
    assert(userAccountInfo.balance.eq(new BN(50_000_000)));

    const tokenBalance = await getTokenBalance(
      pg.connection,
      userTokenAccount
    );
    assert(tokenBalance.eq(new BN(50_000_000)));
  });

  // Helper Functions
  async function getTokenBalance(connection, tokenAccount) {
    const accountInfo = await connection.getTokenAccountBalance(tokenAccount);
    return new BN(accountInfo.value.amount);
  }

  async function createTokenAccount(connection, payer, mint, owner) {
    const account = web3.Keypair.generate();
    const lamports = await connection.getMinimumBalanceForRentExemption(165);

    const transaction = new web3.Transaction().add(
      web3.SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: account.publicKey,
        space: 165,
        lamports,
        programId: TOKEN_PROGRAM_ID,
      }),
      web3.SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: account.publicKey,
        lamports: 1,
      })
    );

    await web3.sendAndConfirmTransaction(connection, transaction, [payer, account]);
    return account.publicKey;
  }

  async function createMint(connection, payer, mintAuthority, freezeAuthority, decimals) {
    const mint = web3.Keypair.generate();
    const lamports = await connection.getMinimumBalanceForRentExemption(82);

    const transaction = new web3.Transaction().add(
      web3.SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: mint.publicKey,
        space: 82,
        lamports,
        programId: TOKEN_PROGRAM_ID,
      })
    );

    await web3.sendAndConfirmTransaction(connection, transaction, [payer, mint]);
    return mint.publicKey;
  }
});
