# cl0r0 Rocq 形式化 — Phase 0–3 建置
# 用法:make          建定理庫(mirror → abstract_ars)
#      make reconcile 差分校對(Rust 實例 ↔ Rocq 計算,生成 reconcile_gen.v 並複驗)
#      make clean

COQC    ?= coqc
COQFLAGS ?= -Q theories Cl0r0 -w -deprecated-native-compiler-option -w -native-compiler-disabled

THEORIES := theories/Mirror.v theories/WCRUtil.v theories/RedEdges.v theories/AbstractArs.v theories/ConcreteSN.v theories/ConcreteWCR.v

all: $(THEORIES:.v=.vo)

%.vo: %.v
	$(COQC) $(COQFLAGS) $<

reconcile: all
	cd .. && python3 tools/rocq_reconcile.py

clean:
	rm -f theories/*.vo theories/*.vos theories/*.vok theories/.*.aux theories/reconcile_gen.v

.PHONY: all reconcile clean
