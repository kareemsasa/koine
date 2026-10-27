#!/usr/bin/env python3
"""Generate reviewed bootstrap artifacts from the finite approved catalog.

No database is opened. Generated SQL and Rust manifests are committed and reviewed.
The explicit constraints below elaborate the prose, not executable upgrade paths.
"""
from pathlib import Path
import re
ROOT=Path(__file__).resolve().parents[1]
text=(ROOT/'docs/sqlite-schema-proposal.md').read_text()
tables={}; keys={}
for line in text.splitlines():
    m=re.match(r'\| `(\w+)` \| (.*?) \| (.*?) \|$',line)
    if not m:continue
    name,columns,_=m.groups()
    fields=re.findall(r'`(\w+) (ID|T|B|I|U)(\?)?(?:→(\w+))?`',columns)
    if not fields:continue
    if '**PK' in columns:
        pk=re.search(r'\*\*PK `([^`]+)`',columns)[1]
        keys[name]=pk.strip('()').split(',') if pk.startswith('(') else [pk.split()[0]]
    else:
        fields.insert(0,('id','ID','',''));keys[name]=['id']
    tables[name]=fields
subjects=[('id','ID','',''),('dimension','T','',''),('authority_scope_id','ID','','authority_scopes')]
for field,table in [('game','games'),('entry','source_entries'),('entitlement','entitlements'),('installation','installations'),('target','launch_targets'),('game_link','game_links'),('equivalence','entry_equivalences'),('context_equivalence','context_equivalences'),('target_installation_link','target_installation_links')]:subjects.append((field+'_id','ID','?',table))
tables['claim_subjects']=subjects;keys['claim_subjects']=['id']
tables['claims']=[('id','ID','',''),('subject_id','ID','','claim_subjects'),('dimension','T','',''),('authority_scope_id','ID','','authority_scopes'),('evidence_id','ID','','evidence_records'),('collection_id','ID','','collections'),('knowledge','T','',''),('value_text','T','?',''),('value_bytes','B','?',''),('access_kind','T','?',''),('reason','T','?',''),('basis_code','T','',''),('negative_justification_id','ID','?','negative_justifications'),('coverage_part_id','ID','?','coverage_parts')]
keys['claims']=['id']
tables['database_identity']=[('id','ID','',''),('family','T','',''),('initial_version','I','','')];keys['database_identity']=['id']
tables['migration_history']=[('id','ID','',''),('migration_id','T','',''),('source_version','I','',''),('target_version','I','',''),('artifact_digest','B','',''),('build','T','',''),('applied_at','U','','')];keys['migration_history']=['id']
unique={
'database_identity':['family'], 'rule_versions':['kind,name,version'], 'identity_profiles':['namespace,version','id,namespace,scope_kind'], 'identity_scopes':['id,namespace,scope_kind'],
'entry_keys':['namespace,scope_id,external_key','id,namespace,scope_kind'], 'entry_key_observations':['collection_id,item_key','id,collection_id'],
'game_links':['game_id,entry_id'], 'entry_equivalences':['left_entry_id,right_entry_id'], 'context_equivalences':['left_context_id,right_context_id'],
'entitlements':['entry_id,subject_context_id,authority_scope_id,entitlement_key','id,authority_scope_id'],
'installations':['entry_id,manager_context_id,machine_context_id,instance_key','id,manager_context_id'],
'launch_targets':['entry_id,launcher_context_id,target_key','id,launcher_context_id'], 'target_installation_links':['target_id,installation_id'],
'authority_scopes':['authority_namespace,scope_key'], 'requested_units':['collection_id,unit_key','id,collection_id','id,collection_id,dimension,authority_scope_id'],
'coverage_results':['unit_id','id,collection_id,unit_id,dimension,authority_scope_id'],
'coverage_parts':['coverage_id,part_key','id,collection_id','id,collection_id,dimension,authority_scope_id','id,coverage_id,collection_id,unit_id,dimension,authority_scope_id','id,collection_id,outcome,dimension,authority_scope_id'],
'evidence_records':['collection_id,item_key','id,collection_id'], 'diagnostics':['collection_id,item_key'],
'claim_subjects':['id,dimension,authority_scope_id'], 'claims':['id,collection_id','id,dimension','id,subject_id,dimension,authority_scope_id'],
'negative_justifications':['id,subject_id,dimension,authority_scope_id,collection_id'], 'handoff_records':['attempt_id,item_key'], 'process_observations':['attempt_id,item_key','id,attempt_id,state'], 'migration_history':['source_version','target_version']}
checks={t:[] for t in tables}; fks={t:[] for t in tables}
def enum(t,c,values):checks[t].append(f"{c} IN ({','.join(repr(x) for x in values.split(','))})")
def fk(t,cols,parent,targets):fks[t].append((cols,parent,targets))
def ck(t,expr):checks[t].append(expr)
enum('rule_versions','kind','identity,interpretation,reconciliation,ordering,freshness,transport')
enum('contexts','kind','subject,manager,machine,library,source_instance');enum('contexts','resolution','identified,provisional')
ck('contexts',"(resolution='identified' AND external_key IS NOT NULL) OR (resolution='provisional' AND external_key IS NULL)")
for t in ['identity_profiles','identity_scopes']:enum(t,'scope_kind','global,account,instance,account_instance,library')
enum('identity_profiles','key_role','catalog,reporter_local,manual')
ck('identity_profiles',"(key_role='catalog' AND catalog_namespace IS NOT NULL) OR (key_role!='catalog' AND catalog_namespace IS NULL)")
enum('identity_scopes','resolution','resolved,provisional')
shape={'global':[], 'account':['subject_context_id'],'instance':['instance_context_id'],'account_instance':['subject_context_id','instance_context_id'],'library':['library_context_id']}
roles=['subject_context_id','instance_context_id','library_context_id']
branches=[]
for kind,present in shape.items():
    branches.append("(resolution='resolved' AND scope_kind="+repr(kind)+" AND provisional_token IS NULL AND "+' AND '.join(c+(' IS NOT NULL' if c in present else ' IS NULL') for c in roles)+')')
# Provisional scopes may preserve only compatible role components.
for kind,present in shape.items():
    if kind=='global':continue
    branches.append("(resolution='provisional' AND scope_kind="+repr(kind)+" AND provisional_token IS NOT NULL AND "+' AND '.join(c+' IS NULL' for c in roles if c not in present)+')')
ck('identity_scopes',' OR '.join(branches))
fk('entry_keys','scope_id,namespace,scope_kind','identity_scopes','id,namespace,scope_kind')
for cols,parent,targets in [('key_id,namespace,scope_kind','entry_keys','id,namespace,scope_kind'),('profile_id,namespace,scope_kind','identity_profiles','id,namespace,scope_kind'),('evidence_id,collection_id','evidence_records','id,collection_id'),('coverage_part_id,collection_id','coverage_parts','id,collection_id')]:fk('entry_key_observations',cols,parent,targets)
for t,l,r in [('entry_equivalences','left_entry_id','right_entry_id'),('context_equivalences','left_context_id','right_context_id')]:ck(t,f'{l} < {r}')
fk('target_installation_links','target_id,manager_context_id','launch_targets','id,launcher_context_id');fk('target_installation_links','installation_id,manager_context_id','installations','id,manager_context_id')
enum('collections','kind','discovery,manual');ck('collections',"request_protocol='koine.request.v1'")
ck('requested_units','(selector IS NULL AND selector_rule_id IS NULL) OR (selector IS NOT NULL AND selector_rule_id IS NOT NULL)')
fk('coverage_results','unit_id,collection_id,dimension,authority_scope_id','requested_units','id,collection_id,dimension,authority_scope_id')
fk('coverage_parts','coverage_id,collection_id,unit_id,dimension,authority_scope_id','coverage_results','id,collection_id,unit_id,dimension,authority_scope_id')
fk('coverage_parts','parent_part_id,coverage_id,collection_id,unit_id,dimension,authority_scope_id','coverage_parts','id,coverage_id,collection_id,unit_id,dimension,authority_scope_id')
for t in ['coverage_results','coverage_parts','collection_closures']:
    enum(t,'outcome','complete,partial,failed,inconclusive');ck(t,"outcome='complete' OR reason IS NOT NULL")
ck('coverage_parts','parent_part_id IS NULL OR parent_part_id != id')
ck('coverage_parts',"(parent_part_id IS NULL AND selector_kind='whole_unit' AND selector IS NULL AND selector_rule_id IS NULL) OR (parent_part_id IS NOT NULL AND selector_kind IN ('bounded','residual') AND selector IS NOT NULL AND selector_rule_id IS NOT NULL)")
enum('coverage_parts','inspection','inspected,uninspected,mixed,unknown')
ck('coverage_parts',"(outcome!='complete' OR inspection='inspected') AND (inspection NOT IN ('uninspected','unknown') OR outcome IN ('failed','inconclusive')) AND (inspection!='mixed' OR outcome!='complete')")
ck('collection_closures',"result_protocol='koine.result.v1'")
enum('evidence_records','origin_kind','external,user_assertion');enum('evidence_records','locator_kind','path,opaque,unavailable')
ck('evidence_records',"(locator_kind='unavailable' AND locator IS NULL) OR (locator_kind!='unavailable' AND locator IS NOT NULL)")
enum('diagnostics','severity','info,warning,error')
targets={'collection':None,'unit':'unit_id','coverage_part':'coverage_part_id','evidence':'evidence_id','key_observation':'key_observation_id','claim':'claim_id'}
ck('diagnostics',' OR '.join('('+"target_kind="+repr(k)+' AND '+' AND '.join(c+(' IS NOT NULL' if c==v else ' IS NULL') for c in targets.values() if c)+')' for k,v in targets.items()))
for col,parent in [('unit_id','requested_units'),('coverage_part_id','coverage_parts'),('evidence_id','evidence_records'),('key_observation_id','entry_key_observations'),('claim_id','claims')]:fk('diagnostics',col+',collection_id',parent,'id,collection_id')
dimensions={'game_label':('game_id',None),'entry_title':('entry_id',None),'entry_edition':('entry_id',None),'manual_visibility':('entry_id',['visible','hidden']),'access':('entitlement_id',['accessible']),'presence':('installation_id',['present']),'completion':('installation_id',['complete','incomplete']),'location':('installation_id',[]),'handoff_capability':('target_id',['supported','unavailable']),'target_installation':('target_installation_link_id',['linked','unlinked']),'game_link':('game_link_id',['linked','unlinked']),'entry_equivalence':('equivalence_id',['equivalent','different']),'context_equivalence':('context_equivalence_id',['equivalent','different'])}
cols=[c for c,_,_,_ in subjects if c not in ['id','dimension','authority_scope_id']]
ck('claim_subjects',' OR '.join('('+'dimension='+repr(d)+' AND '+' AND '.join(c+(' IS NOT NULL' if c==col else ' IS NULL') for c in cols)+')' for d,(col,_) in dimensions.items()))
fk('claim_subjects','entitlement_id,authority_scope_id','entitlements','id,authority_scope_id')
fk('claims','subject_id,dimension,authority_scope_id','claim_subjects','id,dimension,authority_scope_id')
fk('claims','evidence_id,collection_id','evidence_records','id,collection_id');fk('claims','coverage_part_id,collection_id,dimension,authority_scope_id','coverage_parts','id,collection_id,dimension,authority_scope_id')
fk('claims','negative_justification_id,subject_id,dimension,authority_scope_id,collection_id','negative_justifications','id,subject_id,dimension,authority_scope_id,collection_id')
enum('claims','dimension',','.join(dimensions));enum('claims','knowledge','observed,confirmed_negative,unknown')
observed=[]
for d,(_,vals) in dimensions.items():
    payload="value_text IS NOT NULL AND value_bytes IS NULL" if vals is None else ("value_text IS NULL AND value_bytes IS NOT NULL AND length(value_bytes)>0" if not vals else "value_text IS NOT NULL AND value_text IN ("+','.join(repr(v) for v in vals)+") AND value_bytes IS NULL")
    observed.append('('+"dimension="+repr(d)+' AND '+payload+')')
ck('claims',"(knowledge='unknown' AND reason IS NOT NULL AND value_text IS NULL AND value_bytes IS NULL AND access_kind IS NULL AND negative_justification_id IS NULL) OR (knowledge='observed' AND negative_justification_id IS NULL AND ("+' OR '.join(observed)+")) OR (knowledge='confirmed_negative' AND negative_justification_id IS NOT NULL AND value_bytes IS NULL AND value_text IS NOT NULL AND ((dimension='access' AND value_text='inaccessible') OR (dimension='presence' AND value_text='absent')))")
ck('claims',"access_kind IS NULL OR (knowledge='observed' AND dimension='access' AND value_text='accessible')")
for cols,parent,targets in [('subject_id,dimension,authority_scope_id','claim_subjects','id,dimension,authority_scope_id'),('evidence_id,collection_id','evidence_records','id,collection_id'),('coverage_part_id,collection_id,required_outcome,dimension,authority_scope_id','coverage_parts','id,collection_id,outcome,dimension,authority_scope_id')]:fk('negative_justifications',cols,parent,targets)
ck('negative_justifications',"(kind='explicit' AND coverage_part_id IS NULL AND required_outcome IS NULL) OR (kind='exhaustive_omission' AND coverage_part_id IS NOT NULL AND required_outcome IS NOT NULL AND required_outcome='complete')")
enum('reconciliation_decisions','kind','supersession,supersession_rejected,dispute');ck('reconciliation_decisions',"kind!='supersession' OR ordering_rule_id IS NOT NULL")
enum('decision_claims','role','older,replacement,identity_support,ordering_support,negative_support,conflicting')
ck('supersessions','older_claim_id != replacement_claim_id')
for col in ['older_claim_id','replacement_claim_id']:fk('supersessions',col+',dimension','claims','id,dimension')
ck('decision_seals',"protocol='koine.decision.v1'")
enum('handoff_records','issuance','issued,not_issued,unknown');enum('handoff_records','disposition','pending,accepted,failed,inconclusive,not_requested')
ck('handoff_records',"(disposition NOT IN ('pending','accepted') OR issuance='issued') AND (disposition!='not_requested' OR issuance='not_issued') AND (disposition NOT IN ('failed','inconclusive','not_requested') OR reason IS NOT NULL)")
ck('handoff_records',"(exit_status IS NULL OR (exit_status>=0 AND termination_signal IS NULL AND disposition IN ('accepted','failed','inconclusive'))) AND (termination_signal IS NULL OR (termination_signal>0 AND exit_status IS NULL AND disposition IN ('accepted','failed','inconclusive')))")
enum('process_observations','state','observed,inconclusive,exited')
ck('process_observations',"(state IN ('observed','exited') AND identity_token IS NOT NULL AND length(identity_token)>0 AND identity_basis IS NOT NULL) OR (state='inconclusive' AND reason IS NOT NULL AND identity_token IS NULL AND identity_basis IS NULL AND pid IS NULL)")
ck('process_observations','pid IS NULL OR pid>0');ck('process_observations',"(prior_observed_id IS NULL AND prior_state IS NULL) OR (prior_observed_id IS NOT NULL AND prior_state IS NOT NULL AND prior_state='observed' AND prior_observed_id!=id)");ck('process_observations',"state!='exited' OR prior_observed_id IS NOT NULL")
fk('process_observations','prior_observed_id,attempt_id,prior_state','process_observations','id,attempt_id,state')
ck('database_identity',"family='koine.offline.foundation' AND initial_version=1")
ck('migration_history',"migration_id='bootstrap.v1' AND source_version=0 AND target_version=1")
out=['-- Bootstrap v1 only. Generated by tools/generate_schema.py; no released upgrades.']
for table,fields in tables.items():
    parts=[]
    for col,ty,optional,parent in fields:
        sqlty={'ID':'TEXT','T':'TEXT','B':'BLOB','I':'INTEGER','U':'INTEGER'}[ty]
        part=f'  {col} {sqlty}'+('' if optional else ' NOT NULL')
        if ty=='ID':part+=f" CHECK ({col} IS NULL OR (length({col})=36 AND length(CAST({col} AS BLOB))=36 AND substr({col},9,1)='-' AND substr({col},14,1)='-' AND substr({col},19,1)='-' AND substr({col},24,1)='-' AND length(replace({col},'-',''))=32 AND replace({col},'-','') NOT GLOB '*[^0-9a-f]*'))"
        if ty=='T':part+=f' CHECK ({col} IS NULL OR length({col})>0)'
        if col.endswith('digest'):part+=f' CHECK (length({col})=32)'
        if parent:part+=f' REFERENCES {parent}({"decision_id" if parent=="decision_seals" else "id"}) ON UPDATE RESTRICT ON DELETE RESTRICT'
        parts.append(part)
    parts.append('  PRIMARY KEY ('+','.join(keys[table])+')')
    for u in unique.get(table,[]):parts.append('  UNIQUE ('+u+')')
    for ckexpr in checks[table]:parts.append('  CHECK ('+ckexpr+')')
    for cols,parent,targets in fks[table]:parts.append(f'  FOREIGN KEY ({cols}) REFERENCES {parent}({targets}) ON UPDATE RESTRICT ON DELETE RESTRICT')
    if table=='reconciliation_decisions':parts.append('  FOREIGN KEY (id) REFERENCES decision_seals(decision_id) DEFERRABLE INITIALLY DEFERRED')
    out.append('CREATE TABLE '+table+' (\n'+',\n'.join(parts)+'\n) STRICT;')
    for col,_,_,parent in fields:
        if parent:out.append(f'CREATE INDEX {table}_{col}_fk ON {table}({col});')
    for n,(cols,_,_) in enumerate(fks[table]):out.append(f'CREATE INDEX {table}_composite_{n}_fk ON {table}({cols});')
    for op in ['UPDATE','DELETE']:out.append(f"CREATE TRIGGER {table}_immutable_{op.lower()} BEFORE {op} ON {table} BEGIN SELECT RAISE(ABORT,'immutable record'); END;")
    # Replacement inserts are blocked even when recursive triggers are disabled.
    keytest=' AND '.join(f'{c}=NEW.{c}' for c in keys[table])
    out.append(f"CREATE TRIGGER {table}_duplicate BEFORE INSERT ON {table} WHEN EXISTS(SELECT 1 FROM {table} WHERE {keytest}) BEGIN SELECT RAISE(ABORT,'duplicate immutable identity'); END;")
# Block REPLACE conflicts through alternate unique keys as well.
for t,groups in unique.items():
    for i,group in enumerate(groups):out.append(f"CREATE TRIGGER {t}_unique_insert_{i} BEFORE INSERT ON {t} WHEN EXISTS(SELECT 1 FROM {t} WHERE "+' AND '.join(f'{c}=NEW.{c}' for c in group.split(','))+ ") BEGIN SELECT RAISE(ABORT,'duplicate immutable key'); END;")
out.append("CREATE UNIQUE INDEX contexts_identified ON contexts(kind,namespace,external_key) WHERE resolution='identified';")
for kind,cols in shape.items():out.append(f"CREATE UNIQUE INDEX scope_{kind} ON identity_scopes(namespace{',' if cols else ''}{','.join(cols)}) WHERE resolution='resolved' AND scope_kind='{kind}';")
out.append("CREATE UNIQUE INDEX scope_provisional ON identity_scopes(provisional_token) WHERE resolution='provisional';")
out.append('CREATE UNIQUE INDEX coverage_root ON coverage_parts(coverage_id) WHERE parent_part_id IS NULL;')
for col in [c for c,_,_,_ in subjects if c.endswith('_id') and c!='authority_scope_id']:
    out.append(f'CREATE UNIQUE INDEX subject_{col} ON claim_subjects({col},dimension,authority_scope_id) WHERE {col} IS NOT NULL;')
for table in tables:
    if table not in ['collections','collection_closures'] and any(c=='collection_id' for c,_,_,_ in tables[table]):
        out.append(f"CREATE TRIGGER {table}_closed BEFORE INSERT ON {table} WHEN EXISTS(SELECT 1 FROM collection_closures WHERE collection_id=NEW.collection_id) BEGIN SELECT RAISE(ABORT,'closed collection'); END;")
for table in ['decision_claims','supersessions']:
    out.append(f"CREATE TRIGGER {table}_sealed BEFORE INSERT ON {table} WHEN EXISTS(SELECT 1 FROM decision_seals WHERE decision_id=NEW.decision_id) BEGIN SELECT RAISE(ABORT,'sealed decision'); END;")
out.append("""CREATE TRIGGER supersession_cycle BEFORE INSERT ON supersessions WHEN EXISTS(
 WITH RECURSIVE reachable(id) AS (VALUES(NEW.replacement_claim_id) UNION SELECT s.replacement_claim_id FROM supersessions s JOIN reachable r ON s.older_claim_id=r.id)
 SELECT 1 FROM reachable WHERE id=NEW.older_claim_id
) BEGIN SELECT RAISE(ABORT,'supersession cycle'); END;""")
out.append("""CREATE TRIGGER coverage_cycle BEFORE INSERT ON coverage_parts WHEN NEW.parent_part_id IS NOT NULL AND (
 NOT EXISTS(SELECT 1 FROM coverage_parts WHERE id=NEW.parent_part_id) OR EXISTS(
 WITH RECURSIVE ancestors(id,path,cycle) AS (
 SELECT NEW.parent_part_id,'|'||NEW.parent_part_id||'|',0
 UNION ALL SELECT p.parent_part_id,a.path||p.parent_part_id||'|',instr(a.path,'|'||p.parent_part_id||'|')>0
 FROM coverage_parts p JOIN ancestors a ON p.id=a.id WHERE p.parent_part_id IS NOT NULL AND a.cycle=0)
 SELECT 1 FROM ancestors WHERE id=NEW.id OR cycle=1)) BEGIN SELECT RAISE(ABORT,'coverage parent'); END;""")
for t,colset in [('claims','subject_id,evidence_id'),('evidence_records','collection_id,collected_at'),('supersessions','replacement_claim_id,older_claim_id'),('handoff_records','attempt_id,recorded_at'),('process_observations','attempt_id,observed_at')]:out.append(f'CREATE INDEX {t}_retrieval ON {t}({colset});')
(ROOT/'crates/koine-sqlite/src/bootstrap_v1.sql').write_text('\n\n'.join(out)+'\n')
# Exact canonical manifests are read only from their frozen tables in the document.
manifest=[]
for name,start,end in [('DECISION','### Decision manifest:','### Request manifest:'),('REQUEST','### Request manifest:','### Result manifest:'),('RESULT','### Result manifest:','### Scope-key framing')]:
    section=text.split(start)[1].split(end)[0];rows=[]
    for line in section.splitlines():
        m=re.match(r'\| (\w+) \| ([a-z_, ]+) \| ([a-z_, ]+) \|$',line)
        if m:rows.append(m.groups())
    assert rows
    manifest.append('pub const '+name+': &[Manifest] = &[\n'+''.join('Manifest { table: '+repr(t).replace("'",'"')+', columns: &['+','.join('"'+c.strip()+'"' for c in cs.split(','))+'], keys: &['+','.join('"'+c.strip()+'"' for c in ks.split(','))+'] },\n' for t,cs,ks in rows)+'];')
manifest.append('pub const CATALOG: &[Manifest] = &[\n'+''.join('Manifest { table: "'+t+'", columns: &['+','.join('"'+c+'"' for c,_,_,_ in fs)+'], keys: &['+','.join('"'+k+'"' for k in keys[t])+'] },\n' for t,fs in tables.items())+'];')
(ROOT/'crates/koine-core/src/manifests.rs').write_text('// Generated frozen manifests and finite record catalog.\n'+'\n'.join(manifest)+'\n')
print(f'Generated {len(tables)} STRICT tables and frozen manifests.')
