import { Principal } from '@dfinity/principal';

export class NodeMetrics {
  date: Date;
  num_block_failures_total: bigint;
  node_id: Principal;
  num_blocks_proposed_total: bigint;
  subnet_id: Principal;

  constructor(
    ts: bigint,
    num_block_failures_total: bigint,
    node_id: Principal,
    num_blocks_proposed_total: bigint,
    subnet_id: Principal
  ) {
    this.date = new Date(Number(ts / BigInt(1e6)));
    this.num_block_failures_total = num_block_failures_total;
    this.node_id = node_id;
    this.num_blocks_proposed_total = num_blocks_proposed_total;
    this.subnet_id = subnet_id;
  }
}
