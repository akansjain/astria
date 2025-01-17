use std::collections::HashMap;

use astria_sequencer_relayer::{
    base64_string::Base64String,
    data_availability::CelestiaClientBuilder,
    sequencer_block::{
        get_namespace,
        IndexedTransaction,
        SequencerBlock,
        DEFAULT_NAMESPACE,
    },
    types::{
        BlockId,
        Commit,
        Parts,
    },
};
use astria_sequencer_relayer_test::init_test;
use ed25519_consensus::{
    SigningKey,
    VerificationKey,
};
use rand_core::OsRng;

fn empty_commit() -> Commit {
    Commit {
        height: "0".to_string(),
        round: 0,
        block_id: BlockId {
            hash: Base64String(vec![]),
            part_set_header: Parts {
                total: 0,
                hash: Base64String(vec![]),
            },
        },
        signatures: vec![],
    }
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn get_latest_height() {
    let test_env = init_test().await;
    let bridge_endpoint = test_env.bridge_endpoint();
    let client = CelestiaClientBuilder::new(bridge_endpoint).build().unwrap();
    let height = client.get_latest_height().await.unwrap();
    assert!(height > 0);
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn get_blocks_public_key_filter() {
    // test that get_blocks only gets blocked signed with a specific key

    let test_env = init_test().await;
    let bridge_endpoint = test_env.bridge_endpoint();
    let client = CelestiaClientBuilder::new(bridge_endpoint).build().unwrap();

    let tx = Base64String(b"noot_was_here".to_vec());

    let block_hash = Base64String(vec![99; 32]);
    let block = SequencerBlock {
        block_hash: block_hash.clone(),
        header: Default::default(),
        last_commit: empty_commit(),
        sequencer_txs: vec![IndexedTransaction {
            block_index: 0,
            transaction: tx.clone(),
        }],
        rollup_txs: HashMap::new(),
    };

    println!("submitting block");
    let signing_key = SigningKey::new(OsRng);
    let verification_key = VerificationKey::from(&signing_key);
    let submit_block_resp = client
        .submit_block(block, &signing_key, verification_key)
        .await
        .unwrap();
    let height = submit_block_resp
        .namespace_to_block_num
        .get(&DEFAULT_NAMESPACE.to_string())
        .unwrap();

    // generate new, different key
    let signing_key = SigningKey::new(OsRng);
    let verification_key = VerificationKey::from(&signing_key);
    println!("getting blocks");
    let resp = client
        .get_blocks(*height, Some(verification_key))
        .await
        .unwrap();
    assert!(resp.is_empty());
}

#[tokio::test]
#[ignore = "very slow init of test environment"]
async fn celestia_client() {
    // test submit_block
    let test_env = init_test().await;
    let bridge_endpoint = test_env.bridge_endpoint();
    let client = CelestiaClientBuilder::new(bridge_endpoint).build().unwrap();

    let tx = Base64String(b"noot_was_here".to_vec());
    let secondary_namespace = get_namespace(b"test_namespace");
    let secondary_tx = Base64String(b"noot_was_here_too".to_vec());

    let block_hash = Base64String(vec![99; 32]);
    let mut block = SequencerBlock {
        block_hash: block_hash.clone(),
        header: Default::default(),
        last_commit: empty_commit(),
        sequencer_txs: vec![IndexedTransaction {
            block_index: 0,
            transaction: tx.clone(),
        }],
        rollup_txs: HashMap::new(),
    };
    block.rollup_txs.insert(
        secondary_namespace.clone(),
        vec![IndexedTransaction {
            block_index: 1,
            transaction: secondary_tx.clone(),
        }],
    );

    let signing_key = SigningKey::new(OsRng);
    let verification_key = VerificationKey::from(&signing_key);

    let submit_block_resp = client
        .submit_block(block, &signing_key, verification_key)
        .await
        .unwrap();
    let height = submit_block_resp
        .namespace_to_block_num
        .get(&DEFAULT_NAMESPACE.to_string())
        .unwrap();

    // test check_block_availability
    let resp = client.check_block_availability(*height).await.unwrap();
    assert_eq!(resp.height, *height);

    // test get_blocks
    let resp = client
        .get_blocks(*height, Some(verification_key))
        .await
        .unwrap();
    assert_eq!(resp.len(), 1);
    assert_eq!(resp[0].block_hash, block_hash);
    assert_eq!(resp[0].header, Default::default());
    assert_eq!(resp[0].sequencer_txs.len(), 1);
    assert_eq!(resp[0].sequencer_txs[0].block_index, 0);
    assert_eq!(resp[0].sequencer_txs[0].transaction, tx);
    assert_eq!(resp[0].rollup_txs.len(), 1);
    assert_eq!(resp[0].rollup_txs[&secondary_namespace][0].block_index, 1);
    assert_eq!(
        resp[0].rollup_txs[&secondary_namespace][0].transaction,
        secondary_tx
    );
}
