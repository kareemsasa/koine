#!/usr/bin/env python3
"""Independent Python reference encoder for frozen canonical bundle vectors.

These canonical-only samples do not claim semantic validity as database fixtures.
No Rust implementation is imported or executed; hashlib and struct are stdlib.
"""
from pathlib import Path
import hashlib,re,struct
root=Path(__file__).resolve().parents[1]
def uid(n):return f'00000000-0000-4000-8000-{n:012x}'
def encode(value):
    if value is None:return b'\0'
    if isinstance(value,int):return b'\1'+struct.pack('>q',value)
    if isinstance(value,str):
        value=value.encode('utf-8');return b'\2'+struct.pack('>Q',len(value))+value
    if isinstance(value,bytes):return b'\3'+struct.pack('>Q',len(value))+value
    return b'\4'+struct.pack('>Q',len(value))+b''.join(map(encode,value))
def frame(protocol,payload):return b'KOIN\0\1'+encode(protocol)+encode(payload)
records=[
('collections',dict(id=uid(1),kind='discovery',reporter='synthetic',reporter_version='1',reporter_context_id=uid(2),interpretation_rule_id=uid(3),started_at=-9,request_protocol='koine.request.v1')),
('requested_units',dict(id=uid(4),collection_id=uid(1),unit_key='root-A/presence',authority_scope_id=uid(5),dimension='presence',root_context_id=uid(6),selector=None,selector_rule_id=None)),
('coverage_results',dict(id=uid(7),unit_id=uid(4),collection_id=uid(1),dimension='presence',authority_scope_id=uid(5),outcome='complete',reason=None)),
('coverage_parts',dict(id=uid(8),coverage_id=uid(7),collection_id=uid(1),unit_id=uid(4),dimension='presence',authority_scope_id=uid(5),part_key='root',parent_part_id=None,selector_kind='whole_unit',selector=None,selector_rule_id=None,inspection='inspected',outcome='complete',reason=None)),
('evidence_records',dict(id=uid(9),collection_id=uid(1),origin_kind='external',origin_namespace='fixture:steam',locator_kind='path',locator=b'/synthetic/\xff',collected_at=-8,source_effective_at=None,source_revision='sequence:2',valid_until=None,interpretation_rule_id=uid(3),item_key='evidence')),
('claims',dict(id=uid(10),subject_id=uid(11),dimension='presence',authority_scope_id=uid(5),evidence_id=uid(9),collection_id=uid(1),knowledge='observed',value_text='present',value_bytes=None,access_kind=None,reason=None,basis_code='synthetic',negative_justification_id=None,coverage_part_id=uid(8))),
('reconciliation_decisions',dict(id=uid(12),kind='supersession',rule_id=uid(13),ordering_rule_id=uid(14),assessed_at=-7,reason='Koiné sequence',collection_id=uid(1))),
('decision_claims',dict(decision_id=uid(12),claim_id=uid(15),role='older')),
('decision_claims',dict(decision_id=uid(12),claim_id=uid(10),role='replacement')),
('supersessions',dict(older_claim_id=uid(15),replacement_claim_id=uid(10),dimension='presence',decision_id=uid(12))),
('collection_closures',dict(collection_id=uid(1),outcome='complete',finished_at=-6,reason=None,result_protocol='koine.result.v1')),
]
doc=(root/'docs/sqlite-schema-proposal.md').read_text()
manifests={}
for protocol,start,end in [('koine.request.v1','### Request manifest:','### Result manifest:'),('koine.decision.v1','### Decision manifest:','### Request manifest:'),('koine.result.v1','### Result manifest:','### Scope-key framing')]:
    manifests[protocol]=[m.groups() for line in doc.split(start)[1].split(end)[0].splitlines() if (m:=re.match(r'\| (\w+) \| ([a-z_, ]+) \| ([a-z_, ]+) \|$',line))]
def bundle(protocol):
    payload=[]
    for table,columns,keys in manifests[protocol]:
        columns=columns.split(', ');keys=keys.split(', ')
        rows=[row for name,row in records if name==table]
        rows.sort(key=lambda row:tuple(encode(row.get(k)) for k in keys))
        payload.append([table,columns,[[row.get(c) for c in columns] for row in rows]])
    return frame(protocol,payload)
expected=[]
for protocol in ['koine.request.v1','koine.decision.v1','koine.result.v1']:
    data=bundle(protocol);digest=hashlib.sha256(data).digest();owner=uid(12) if protocol=='koine.decision.v1' else uid(1)
    expected.append('\t'.join([protocol,owner,data.hex(),digest.hex()]))
    if protocol=='koine.request.v1':records[0][1]['request_digest']=digest
    if protocol=='koine.decision.v1':records.append(('decision_seals',dict(decision_id=uid(12),protocol=protocol,content_digest=digest)))
    if protocol=='koine.result.v1':next(row for table,row in records if table=='collection_closures')['result_digest']=digest
fixture=[]
for index,(table,row) in enumerate(records):
    for col,v in row.items():
        tag='N' if v is None else 'I' if isinstance(v,int) else 'B' if isinstance(v,bytes) else 'T'
        value='-' if v is None else v.hex() if isinstance(v,bytes) else str(v)
        fixture.append('\t'.join([table,str(index),col,tag,value]))
folder=root/'crates/koine-core/tests/fixtures'
(folder/'bundle-records.tsv').write_text('\n'.join(fixture)+'\n')
(folder/'bundles-v1.tsv').write_text('\n'.join(expected)+'\n')
print('Generated independent request/result/decision canonical vectors.')
