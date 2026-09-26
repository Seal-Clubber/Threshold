#![cfg(feature = "engine-tests")]

//! Runs the compiled Threshold WASM in Ootle's actual local execution engine.
use tari_engine_types::virtual_substate::{VirtualSubstate, VirtualSubstateId};
use tari_ootle_transaction::args;
use tari_template_lib::prelude::{ComponentAddress, Hash32, Scalar32Bytes};
use tari_template_lib::types::{constants::TARI_TOKEN, UtxoAddress};
use tari_template_test_tooling::TemplateTest;
use threshold_template::{Stage, Terms, escrow_root};
use tari_ootle_transaction::Network;
use tari_ootle_wallet_crypto::{MaskAndValue, StealthInputWitness, stealth, kdfs};
use ootle_rs::keys::OotleSecretKey;
use ootle_byte_type::{ToByteType, ConvertFromByteType};
use tari_crypto::{keys::{PublicKey, SecretKey}, ristretto::{RistrettoPublicKey, RistrettoSecretKey}};
use tari_engine_types::substate::SubstateId;
use threshold_client::{private_output, private_transfer, decrypt_output, escrow_input, check_aggregate};
use tari_template_lib::prelude::Amount;
use tari_template_lib::types::stealth::SpendAuthorization;

#[test]
fn frozen_terms_and_consensus_deadline_are_engine_enforced() {
    let mut test = TemplateTest::new(env!("CARGO_MANIFEST_DIR"), ["../threshold-template"]);
    test.set_virtual_substate(VirtualSubstateId::CurrentEpoch, VirtualSubstate::CurrentEpoch(10));
    let recipient = test.to_public_key_bytes();
    let terms = Terms {
        version: 1,
        nonce: Hash32::from_array([17; 32]),
        resource: TARI_TOKEN,
        funding_deadline: 12,
        sponsor_key: recipient,
        stages: vec![Stage {
            leg: 0,
            community: 6_000_000,
            sponsor: 4_000_000,
            recipient,
            reviewers: vec![],
            quorum: 0,
            deadline: 13,
            upfront: true,
        }],
    };
    let address: ComponentAddress = test.call_function("Threshold", "new", args![terms], vec![]);
    let status: (bool, u32, Vec<u32>) = test.call_method(address, "status", args![], vec![]);
    assert_eq!(status, (false, 0, vec![]));

    let no_funding = test.transaction().call_method(address, "activate", args![vec![0u32], vec![Scalar32Bytes::from([0u8; 32])]]).build_and_seal(test.secret_key());
    let rejection = test.execute_expect_failure(no_funding, vec![]);
    assert!(rejection.to_string().contains("all role/stage openings required"));

    test.set_virtual_substate(VirtualSubstateId::CurrentEpoch, VirtualSubstate::CurrentEpoch(12));
    let expired = test.transaction().call_method(address, "activate", args![vec![0u32], Vec::<Scalar32Bytes>::new()]).build_and_seal(test.secret_key());
    let rejection = test.execute_expect_failure(expired, vec![]);
    assert!(rejection.to_string().contains("activation closed"));
}

#[test]
fn confidential_pledge_failed_bundle_and_owner_refund_at_boundary() {
    let mut test=TemplateTest::new(env!("CARGO_MANIFEST_DIR"),["../threshold-template"]);
    test.set_virtual_substate(VirtualSubstateId::CurrentEpoch,VirtualSubstate::CurrentEpoch(10));
    let (account,_,account_secret,account_pub)=test.create_funded_account_with_keypair();
    let wallet=OotleSecretKey::new(Network::LocalNet,account_secret.clone(),RistrettoSecretKey::random(&mut rand::rng()));
    let address=wallet.to_address();
    assert_eq!(*address.account_public_key(),account_pub.to_byte_type());
    let terms=Terms{version:1,nonce:Hash32::from_array([55;32]),resource:TARI_TOKEN,funding_deadline:12,
        sponsor_key:test.to_public_key_bytes(),
        stages:vec![Stage{leg:0,community:3_000_000,sponsor:2_000_000,recipient:account_pub.to_byte_type(),
            reviewers:vec![],quorum:0,deadline:13,upfront:true}]};
    let component:ComponentAddress=test.call_function("Threshold","new",args![terms],vec![]);
    let funding=private_output(&address,3_000_000,None).unwrap();
    let change=private_output(&address,97_000_000,None).unwrap();
    let shield=stealth::create_transfer_statement(Vec::<StealthInputWitness>::new(),Amount::from(100_000_000u64),[&funding,&change],Amount::zero()).unwrap();
    test.execute_expect_success(test.transaction().with_auto_fill_inputs()
        .call_method(account,"withdraw",args![TARI_TOKEN,100_000_000u64])
        .put_last_instruction_output_on_workspace("funds")
        .stealth_transfer_with_input_bucket(TARI_TOKEN,shield.clone(),"funds")
        .build_and_seal(&account_secret),vec![]);
    let nonce=RistrettoPublicKey::convert_from_byte_type(&shield.stealth_outputs()[0].output.sender_public_nonce).unwrap();
    let input_secret=kdfs::owner_stealth_dh_secret(Network::LocalNet,&account_secret,&nonce);
    let funding_open=decrypt_output(&wallet,&shield.stealth_outputs()[0]).unwrap();
    let mut unsafe_cell=private_output(&address,3_000_000,Some(component)).unwrap();
    unsafe_cell.auth=SpendAuthorization::KeyAndScript{spend_key:account_pub.to_byte_type(),condition_root:escrow_root(component)};
    let backdoor=private_transfer(vec![StealthInputWitness::new(funding_open.clone())],&[unsafe_cell]).unwrap();
    let source=SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,backdoor.stealth_inputs()[0].commitment.into()));
    let backdoor_tx=test.transaction().with_auto_fill_inputs().add_input(source.clone())
        .call_method(component,"pledge",args![false,backdoor])
        .finish().add_signer(&account_pub.to_byte_type(),&input_secret).seal(&account_secret);
    assert!(test.execute_expect_failure(backdoor_tx,vec![]).to_string().contains("pure component lock required"));
    let locked=private_output(&address,3_000_000,Some(component)).unwrap();
    let cell=MaskAndValue{mask:locked.witness.mask.clone(),value:locked.witness.amount};
    let pledge=private_transfer(vec![StealthInputWitness::new(funding_open)],&[locked]).unwrap();
    let source=SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,pledge.stealth_inputs()[0].commitment.into()));
    test.execute_expect_success(test.transaction().with_auto_fill_inputs().add_input(source)
        .call_method(component,"pledge",args![false,pledge])
        .finish().add_signer(&account_pub.to_byte_type(),&input_secret).seal(&account_secret),vec![]);
    let community=check_aggregate(&[cell.clone()],3_000_000).unwrap();
    let bad=test.transaction().with_auto_fill_inputs().call_method(component,"activate",
        args![vec![0u32],vec![community,Scalar32Bytes::from([0u8;32])]])
        .build_and_seal(&account_secret);
    assert!(test.execute_expect_failure(bad,vec![]).to_string().contains("missing funding role"));
    let payout=private_output(&address,3_000_000,None).unwrap();
    let refund=private_transfer(vec![escrow_input(cell.clone(),component).unwrap()],&[payout]).unwrap();
    let locked_id=SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,cell.to_commitment().to_byte_type().into()));
    let before=test.transaction().with_auto_fill_inputs().add_input(locked_id.clone())
        .call_method(component,"refund",args![0u32,refund.clone()]).build_and_seal(&account_secret);
    assert!(test.execute_expect_failure(before,vec![]).to_string().contains("funding still open"));
    test.set_virtual_substate(VirtualSubstateId::CurrentEpoch,VirtualSubstate::CurrentEpoch(12));
    let after=test.transaction().with_auto_fill_inputs().add_input(locked_id.clone())
        .call_method(component,"refund",args![0u32,refund.clone()]).build_and_seal(&account_secret);
    test.execute_expect_success(after,vec![]);
    let again=test.transaction().with_auto_fill_inputs().add_input(locked_id)
        .call_method(component,"refund",args![0u32,refund]).build_and_seal(&account_secret);
    assert!(test.execute_expect_failure(again,vec![]).to_string().contains("already refunded"));
}

fn fund_two_cells(test:&mut TemplateTest, component:ComponentAddress, account:ComponentAddress,
    secret:&RistrettoSecretKey, wallet:&OotleSecretKey, values:[u64;2], sponsor:bool)->[MaskAndValue;2] {
    let address=wallet.to_address();
    let value=values[0]+values[1];
    let funding=private_output(&address,value,None).unwrap();
    let change=private_output(&address,100_000_000-value,None).unwrap();
    let shield=stealth::create_transfer_statement(Vec::<StealthInputWitness>::new(),Amount::from(100_000_000u64),[&funding,&change],Amount::zero()).unwrap();
    test.execute_expect_success(test.transaction().with_auto_fill_inputs()
        .call_method(account,"withdraw",args![TARI_TOKEN,100_000_000u64])
        .put_last_instruction_output_on_workspace("funds")
        .stealth_transfer_with_input_bucket(TARI_TOKEN,shield.clone(),"funds")
        .build_and_seal(secret),vec![]);
    let nonce=RistrettoPublicKey::convert_from_byte_type(&shield.stealth_outputs()[0].output.sender_public_nonce).unwrap();
    let input_secret=kdfs::owner_stealth_dh_secret(Network::LocalNet,secret,&nonce);
    let opening=decrypt_output(wallet,&shield.stealth_outputs()[0]).unwrap();
    let outputs=values.map(|v|private_output(&address,v,Some(component)).unwrap());
    let cells=outputs.each_ref().map(|o|MaskAndValue{mask:o.witness.mask.clone(),value:o.witness.amount});
    let pledge=private_transfer(vec![StealthInputWitness::new(opening)],&outputs).unwrap();
    let id=SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,pledge.stealth_inputs()[0].commitment.into()));
    let public=RistrettoPublicKey::from_secret_key(secret).to_byte_type();
    test.execute_expect_success(test.transaction().with_auto_fill_inputs().add_input(id)
        .call_method(component,"pledge",args![sponsor,pledge])
        .finish().add_signer(&public,&input_secret).seal(secret),vec![]);
    cells
}

fn fund_one_cell(test:&mut TemplateTest, component:ComponentAddress, account:ComponentAddress,
    secret:&RistrettoSecretKey, wallet:&OotleSecretKey, value:u64, sponsor:bool)->MaskAndValue {
    let address=wallet.to_address();
    let funding=private_output(&address,value,None).unwrap();
    let shield=stealth::create_transfer_statement(Vec::<StealthInputWitness>::new(),Amount::from(value),[&funding],Amount::zero()).unwrap();
    test.execute_expect_success(test.transaction().with_auto_fill_inputs()
        .call_method(account,"withdraw",args![TARI_TOKEN,value])
        .put_last_instruction_output_on_workspace("funds")
        .stealth_transfer_with_input_bucket(TARI_TOKEN,shield.clone(),"funds")
        .build_and_seal(secret),vec![]);
    let nonce=RistrettoPublicKey::convert_from_byte_type(&shield.stealth_outputs()[0].output.sender_public_nonce).unwrap();
    let input_secret=kdfs::owner_stealth_dh_secret(Network::LocalNet,secret,&nonce);
    let opening=decrypt_output(wallet,&shield.stealth_outputs()[0]).unwrap();
    let locked=private_output(&address,value,Some(component)).unwrap();
    let cell=MaskAndValue{mask:locked.witness.mask.clone(),value:locked.witness.amount};
    let pledge=private_transfer(vec![StealthInputWitness::new(opening)],&[locked]).unwrap();
    let id=SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,pledge.stealth_inputs()[0].commitment.into()));
    let public=RistrettoPublicKey::from_secret_key(secret).to_byte_type();
    test.execute_expect_success(test.transaction().with_auto_fill_inputs().add_input(id)
        .call_method(component,"pledge",args![sponsor,pledge])
        .finish().add_signer(&public,&input_secret).seal(secret),vec![]);
    cell
}

#[test]
fn measured_multi_pledge_activation_and_release_in_engine() {
    let pledge_count=std::env::var("THRESHOLD_SCALE_PLEDGES").ok().and_then(|s|s.parse::<usize>().ok()).unwrap_or(8);
    assert!((2..=32).contains(&pledge_count));
    let started=std::time::Instant::now();
    let mut test=TemplateTest::new(env!("CARGO_MANIFEST_DIR"),["../threshold-template"]);
    test.set_virtual_substate(VirtualSubstateId::CurrentEpoch,VirtualSubstate::CurrentEpoch(10));
    let (community_account,_,community_secret,_)=test.create_funded_account_with_keypair();
    let (sponsor_account,_,sponsor_secret,sponsor_pub)=test.create_funded_account_with_keypair();
    let community=OotleSecretKey::new(Network::LocalNet,community_secret.clone(),RistrettoSecretKey::random(&mut rand::rng()));
    let sponsor=OotleSecretKey::new(Network::LocalNet,sponsor_secret.clone(),RistrettoSecretKey::random(&mut rand::rng()));
    let value_per_pledge=1_000_000u64;
    let community_budget=(pledge_count as u64-1)*value_per_pledge;
    let terms=Terms{version:1,nonce:Hash32::from_array([57;32]),resource:TARI_TOKEN,funding_deadline:12,
        sponsor_key:sponsor_pub.to_byte_type(),stages:vec![Stage{leg:0,community:community_budget,
        sponsor:value_per_pledge,recipient:sponsor_pub.to_byte_type(),reviewers:vec![],quorum:0,deadline:13,upfront:true}]};
    let component:ComponentAddress=test.call_function("Threshold","new",args![terms],vec![]);
    let mut community_cells=Vec::with_capacity(pledge_count-1);
    for _ in 0..pledge_count-1 {
        community_cells.push(fund_one_cell(&mut test,component,community_account,&community_secret,&community,value_per_pledge,false));
    }
    let sponsor_cell=fund_one_cell(&mut test,component,sponsor_account,&sponsor_secret,&sponsor,value_per_pledge,true);
    let funding_elapsed=started.elapsed();
    let masks=vec![check_aggregate(&community_cells,community_budget).unwrap(),
        check_aggregate(&[sponsor_cell.clone()],value_per_pledge).unwrap()];
    let selected=(0..pledge_count as u32).collect::<Vec<_>>();
    test.execute_expect_success(test.transaction().with_auto_fill_inputs()
        .call_method(component,"activate",args![selected,masks]).build_and_seal(&sponsor_secret),vec![]);
    let activation_elapsed=started.elapsed()-funding_elapsed;
    let mut inputs=community_cells.iter().chain(std::iter::once(&sponsor_cell))
        .map(|cell|escrow_input(cell.clone(),component).unwrap()).collect::<Vec<_>>();
    let payout=private_output(&sponsor.to_address(),pledge_count as u64*value_per_pledge,None).unwrap();
    let release=private_transfer(std::mem::take(&mut inputs),&[payout]).unwrap();
    let mut builder=test.transaction().with_auto_fill_inputs();
    for input in release.stealth_inputs() {
        builder=builder.add_input(SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,input.commitment.into())));
    }
    test.execute_expect_success(builder.call_method(component,"release",args![0u32,Hash32::from_array([58;32]),release])
        .build_and_seal(&sponsor_secret),vec![]);
    let status:(bool,u32,Vec<u32>)=test.call_method(component,"status",args![],vec![]);
    assert_eq!(status.0,true);
    assert_eq!(status.1,1);
    assert_eq!(status.2.len(),pledge_count);
    println!("Engine accepted {pledge_count} private pledges, exact activation and {pledge_count}-input release; funding {:?}, activation {:?}, total {:?}",funding_elapsed,activation_elapsed,started.elapsed());
}

#[test]
fn complete_bundle_then_ten_percent_upfront_and_partial_delivery_preserves_refunds() {
    let mut test=TemplateTest::new(env!("CARGO_MANIFEST_DIR"),["../threshold-template"]);
    test.set_virtual_substate(VirtualSubstateId::CurrentEpoch,VirtualSubstate::CurrentEpoch(10));
    let (community_account,_,community_secret,community_pub)=test.create_funded_account_with_keypair();
    let (sponsor_account,_,sponsor_secret,sponsor_pub)=test.create_funded_account_with_keypair();
    let community=OotleSecretKey::new(Network::LocalNet,community_secret.clone(),RistrettoSecretKey::random(&mut rand::rng()));
    let sponsor=OotleSecretKey::new(Network::LocalNet,sponsor_secret.clone(),RistrettoSecretKey::random(&mut rand::rng()));
    let recipient=community_pub.to_byte_type();
    let terms=Terms{version:1,nonce:Hash32::from_array([56;32]),resource:TARI_TOKEN,funding_deadline:12,
        sponsor_key:sponsor_pub.to_byte_type(),stages:vec![
            Stage{leg:0,community:6_000_000,sponsor:4_000_000,recipient,reviewers:vec![],quorum:0,deadline:13,upfront:true},
            Stage{leg:1,community:54_000_000,sponsor:36_000_000,recipient,reviewers:vec![recipient],quorum:1,deadline:15,upfront:false}]};
    let component:ComponentAddress=test.call_function("Threshold","new",args![terms.clone()],vec![]);
    let community_cells=fund_two_cells(&mut test,component,community_account,&community_secret,&community,[6_000_000,54_000_000],false);
    let sponsor_cells=fund_two_cells(&mut test,component,sponsor_account,&sponsor_secret,&sponsor,[4_000_000,36_000_000],true);
    let mut other_terms=terms;
    other_terms.nonce=Hash32::from_array([59;32]);
    let other_component:ComponentAddress=test.call_function("Threshold","new",args![other_terms],vec![]);
    let cross_campaign=private_transfer(vec![escrow_input(community_cells[0].clone(),component).unwrap()],
        &[private_output(&community.to_address(),2_000_000,Some(other_component)).unwrap(),
          private_output(&community.to_address(),4_000_000,Some(other_component)).unwrap()]).unwrap();
    let cross_id=SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,community_cells[0].to_commitment().to_byte_type().into()));
    let cross=test.transaction().with_auto_fill_inputs().add_input(cross_id)
        .call_method(other_component,"pledge",args![false,cross_campaign]).build_and_seal(&community_secret);
    let cross_reason=test.execute_expect_failure(cross,vec![]);
    println!("Cross-campaign escrow reuse rejected: {cross_reason}");
    let masks=vec![
        check_aggregate(&[community_cells[0].clone()],6_000_000).unwrap(),
        check_aggregate(&[sponsor_cells[0].clone()],4_000_000).unwrap(),
        check_aggregate(&[community_cells[1].clone()],54_000_000).unwrap(),
        check_aggregate(&[sponsor_cells[1].clone()],36_000_000).unwrap()];
    let premature=private_transfer(vec![escrow_input(community_cells[0].clone(),component).unwrap(),escrow_input(sponsor_cells[0].clone(),component).unwrap()],
        &[private_output(&community.to_address(),10_000_000,None).unwrap()]).unwrap();
    let mut before_activation=test.transaction().with_auto_fill_inputs();
    for input in premature.stealth_inputs(){before_activation=before_activation.add_input(SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,input.commitment.into())));}
    let reason=test.execute_expect_failure(before_activation.call_method(component,"release",args![0u32,Hash32::from_array([42;32]),premature])
        .build_and_seal(&community_secret),vec![]);
    assert!(reason.to_string().contains("wrong active stage"));
    test.execute_expect_success(test.transaction().with_auto_fill_inputs()
        .call_method(component,"activate",args![vec![0u32,1u32],masks])
        .build_and_seal(&community_secret),vec![]);
    let inputs=vec![escrow_input(community_cells[0].clone(),component).unwrap(),escrow_input(sponsor_cells[0].clone(),component).unwrap()];
    let payout=private_output(&community.to_address(),10_000_000,None).unwrap();
    let release=private_transfer(inputs,&[payout]).unwrap();
    let redirect=private_transfer(vec![escrow_input(community_cells[0].clone(),component).unwrap(),escrow_input(sponsor_cells[0].clone(),component).unwrap()],
        &[private_output(&sponsor.to_address(),10_000_000,None).unwrap()]).unwrap();
    let mut unauthorized=test.transaction().with_auto_fill_inputs();
    for input in redirect.stealth_inputs(){unauthorized=unauthorized.add_input(SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,input.commitment.into())));}
    let unauthorized_reason=test.execute_expect_failure(unauthorized.call_method(component,"release",args![0u32,Hash32::from_array([42;32]),redirect])
        .build_and_seal(&sponsor_secret),vec![]);
    assert!(unauthorized_reason.to_string().contains("recipient must sign payout"));
    let mut builder=test.transaction().with_auto_fill_inputs();
    for input in release.stealth_inputs(){builder=builder.add_input(SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,input.commitment.into())));}
    test.execute_expect_success(builder.call_method(component,"release",args![0u32,Hash32::from_array([42;32]),release.clone()])
        .build_and_seal(&community_secret),vec![]);
    let mut replay=test.transaction().with_auto_fill_inputs();
    for input in release.stealth_inputs(){replay=replay.add_input(SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,input.commitment.into())));}
    let replay_reason=test.execute_expect_failure(replay.call_method(component,"release",args![0u32,Hash32::from_array([42;32]),release])
        .build_and_seal(&community_secret),vec![]);
    println!("Double payout rejected: {replay_reason}");
    let status:(bool,u32,Vec<u32>)=test.call_method(component,"status",args![],vec![]);
    assert_eq!(status,(true,1,vec![0,1]));
    let evidence=Hash32::from_array([43;32]);
    let outsider_vote=test.transaction().with_auto_fill_inputs().call_method(component,"approve",args![1u32,evidence])
        .build_and_seal(&sponsor_secret);
    assert!(test.execute_expect_failure(outsider_vote,vec![]).to_string().contains("not a reviewer"));
    test.execute_expect_success(test.transaction().with_auto_fill_inputs().call_method(component,"approve",args![1u32,evidence])
        .build_and_seal(&community_secret),vec![]);
    let replay_vote=test.transaction().with_auto_fill_inputs().call_method(component,"approve",args![1u32,evidence])
        .build_and_seal(&community_secret);
    assert!(test.execute_expect_failure(replay_vote,vec![]).to_string().contains("already voted"));
    let stage_one=private_transfer(vec![escrow_input(community_cells[1].clone(),component).unwrap(),escrow_input(sponsor_cells[1].clone(),component).unwrap()],
        &[private_output(&community.to_address(),90_000_000,None).unwrap()]).unwrap();
    let mut wrong_evidence=test.transaction().with_auto_fill_inputs();
    for input in stage_one.stealth_inputs(){wrong_evidence=wrong_evidence.add_input(SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,input.commitment.into())));}
    let wrong_reason=test.execute_expect_failure(wrong_evidence.call_method(component,"release",args![1u32,Hash32::from_array([44;32]),stage_one.clone()])
        .build_and_seal(&community_secret),vec![]);
    assert!(wrong_reason.to_string().contains("review quorum not met"));
    let refund_community=private_transfer(vec![escrow_input(community_cells[1].clone(),component).unwrap()],
        &[private_output(&community.to_address(),54_000_000,None).unwrap()]).unwrap();
    let cid=SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,community_cells[1].to_commitment().to_byte_type().into()));
    test.set_virtual_substate(VirtualSubstateId::CurrentEpoch,VirtualSubstate::CurrentEpoch(14));
    let before=test.transaction().with_auto_fill_inputs().add_input(cid.clone())
        .call_method(component,"refund",args![0u32,refund_community.clone()]).build_and_seal(&community_secret);
    assert!(test.execute_expect_failure(before,vec![]).to_string().contains("delivery still open"));
    test.set_virtual_substate(VirtualSubstateId::CurrentEpoch,VirtualSubstate::CurrentEpoch(15));
    let mut too_late=test.transaction().with_auto_fill_inputs();
    for input in stage_one.stealth_inputs(){too_late=too_late.add_input(SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,input.commitment.into())));}
    let late_reason=test.execute_expect_failure(too_late.call_method(component,"release",args![1u32,evidence,stage_one])
        .build_and_seal(&community_secret),vec![]);
    assert!(late_reason.to_string().contains("delivery deadline passed"));
    test.execute_expect_success(test.transaction().with_auto_fill_inputs().add_input(cid)
        .call_method(component,"refund",args![0u32,refund_community]).build_and_seal(&community_secret),vec![]);
    let refund_sponsor=private_transfer(vec![escrow_input(sponsor_cells[1].clone(),component).unwrap()],
        &[private_output(&sponsor.to_address(),36_000_000,None).unwrap()]).unwrap();
    let sid=SubstateId::Utxo(UtxoAddress::new(TARI_TOKEN,sponsor_cells[1].to_commitment().to_byte_type().into()));
    test.execute_expect_success(test.transaction().with_auto_fill_inputs().add_input(sid)
        .call_method(component,"refund",args![1u32,refund_sponsor]).build_and_seal(&sponsor_secret),vec![]);
}
