---
status: accepted
---

# Keep declaration grammar in language adapters

Declaration specs represent target-independent intent, shared capabilities
describe semantic representability, and each language adapter owns complete
target validation and declaration lowering into `CodeBlock`. We reject shared
placement enums, keyword fields, and ordering flags interpreted by generic spec
emitters because they turn the common interface into a union of target grammars
and make a language-local syntax change modify shared code. Pre-0.6.8
declaration syntax configuration may remain as a compatibility lowerer, but it
must not be extended; language adapters may still share private policy-free
lowering helpers. Raw target payloads remain opaque to generic specs. A private
adapter-local recognizer may exist only to freeze an established pre-0.6.8
compatibility case.
