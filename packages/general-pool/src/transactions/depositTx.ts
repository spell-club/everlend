import { TOKEN_PROGRAM_ID } from '@solana/spl-token'
import {
  PublicKey,
  Transaction,
  TransactionCtorFields,
  TransactionInstruction,
} from '@solana/web3.js'
import BN from 'bn.js'
import { Borsh } from '@everlend/common'
import { GeneralPoolsProgram } from '../program'

export class DepositTxData extends Borsh.Data<{ amount: BN }> {
  static readonly SCHEMA = this.struct([
    ['instruction', 'u8'],
    ['amount', 'u64'],
  ])

  instruction = 5
}

type DepositTxParams = {
  registry: PublicKey
  registryPoolConfig: PublicKey
  poolMarket: PublicKey
  pool: PublicKey
  source: PublicKey
  destination: PublicKey
  tokenAccount: PublicKey
  poolMint: PublicKey
  poolMarketAuthority: PublicKey
  amount: BN
}

export class DepositTx extends Transaction {
  constructor(options: TransactionCtorFields, params: DepositTxParams) {
    super(options)
    const { feePayer } = options
    const {
      registry,
      registryPoolConfig,
      poolMarket,
      pool,
      source,
      destination,
      tokenAccount,
      poolMint,
      poolMarketAuthority,
      amount,
    } = params

    const data = DepositTxData.serialize({ amount })

    this.add(
      new TransactionInstruction({
        keys: [
          { pubkey: registry, isSigner: false, isWritable: false },
          { pubkey: registryPoolConfig, isSigner: false, isWritable: false },
          { pubkey: poolMarket, isSigner: false, isWritable: false },
          { pubkey: pool, isSigner: false, isWritable: false },
          { pubkey: source, isSigner: false, isWritable: true },
          { pubkey: destination, isSigner: false, isWritable: true },
          { pubkey: tokenAccount, isSigner: false, isWritable: true },
          { pubkey: poolMint, isSigner: false, isWritable: true },
          { pubkey: poolMarketAuthority, isSigner: false, isWritable: false },
          { pubkey: feePayer, isSigner: true, isWritable: false },
          { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        ],
        programId: GeneralPoolsProgram.PUBKEY,
        data,
      }),
    )
  }
}