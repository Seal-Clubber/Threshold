"""Recheck public bounty receipts and, optionally, live Esmeralda state."""
import argparse
import json
import pathlib
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument(
    "--receipts-only", action="store_true",
    help="check local committed receipts and the manifest without querying Esmeralda",
)
args = parser.parse_args()

ROOT = pathlib.Path(__file__).resolve().parent.parent
EVIDENCE = ROOT / "evidence" / "bounty"
manifest = json.loads((EVIDENCE / "demo-index.json").read_text())
assert manifest["network"] == "esmeralda"
assert sorted(entry["issue"] for entry in manifest["bounties"]) == list(range(900001, 900006))

receipts = {}
for path in EVIDENCE.glob("*-receipt.json"):
    record = json.loads(path.read_text())
    assert record["receipt"]["outcome"] == "Commit", path.name
    receipts[record["transaction_id"]] = path.name

for entry in manifest["bounties"]:
    for transaction_id in entry["transactions"].values():
        assert transaction_id in receipts, f"missing full-commit receipt: {transaction_id}"

if args.receipts_only:
    print(f"Checked {len(receipts)} local full-commit receipts for {len(manifest['bounties'])} bounties; live state not checked")
    raise SystemExit(0)

observed = []
for entry in manifest["bounties"]:
    url = f'{manifest["indexer"]}/substates/{entry["component"]}'
    response = subprocess.run(
        ["curl", "-fsS", "--max-time", "20", url],
        check=True, capture_output=True, text=True,
    )
    data = json.loads(response.stdout)
    assert data["verified"] is True, entry["component"]
    component = data["substate"]["Component"]
    assert component["header"]["template_address"].lower() == manifest["template"].lower()
    state = component["body"]["state"]
    assert state[0] == 2 and state[2] == entry["issue"]
    keys = [item["hex"].lower() for item in state[6]]
    assert keys == [key.lower() for key in entry["reviewers"]]
    assert state[7] == entry["quorum"]
    expected_status, expected_remaining = {
        900001: (4, 0),
        900002: (5, 0),
        900003: (0, 15_000_000_000),
        900004: (0, 15_000_000_000),
        900005: (0, 15_000_000_000),
    }[entry["issue"]]
    assert state[14] == expected_status
    assert state[16] == expected_remaining
    observed.append({
        "component": entry["component"],
        "issue": entry["issue"],
        "verified": True,
        "substate_version": data["version"],
        "reviewer_keys": keys,
        "review_quorum": state[7],
        "award_votes": state[8],
        "no_work_votes": [vote["hex"] if vote else None for vote in state[9]],
        "status_code": state[14],
        "remaining_microtari": state[16],
    })

print(f'Checked {len(observed)} live components and {len(receipts)} full-commit receipts')
print('Source: indexer-verified component reads; not an independent consensus light client')
