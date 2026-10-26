#!/usr/bin/env python3
"""Copies of mod classes for the human harness, written into <out>/bfmod: the modal dialogs the
mod opens with SwingUtilities.invokeLater from its ticks become the harness human's answers
(exporter.HumanPolicy), queued and run right after the tick (HumanPolicy.later / runLater), as the
event queue would run them. Usage: patch_mod.py <mod src dir> <out dir>"""
import os
import sys

src, out = sys.argv[1], sys.argv[2]
os.makedirs(os.path.join(out, "bfmod"), exist_ok=True)


def patch(name, replacements):
    text = open(os.path.join(src, name)).read()
    for old, new in replacements:
        if old not in text:
            sys.exit(f"{name}: not found: {old!r}")
        text = text.replace(old, new)
    open(os.path.join(out, "bfmod", name), "w").write(text)


# Patrocinio.escolher: the three master offers, the human signs one
patch("Patrocinio.java", [
    ("SwingUtilities.invokeLater(() -> escolher(ofertas));", "exporter.HumanPolicy.later(() -> escolherNoHarness(ofertas));"),
    ("   private static synchronized void assinar(Contrato k) {",
     "   private static void escolherNoHarness(List<Contrato> ofertas) {\n"
     "      try {\n"
     "         assinar(ofertas.get(exporter.HumanPolicy.masterSponsor(ofertas.size())));\n"
     "      } finally {\n"
     "         synchronized (Patrocinio.class) {\n"
     "            oferecendo = false;\n"
     "         }\n"
     "      }\n"
     "   }\n\n"
     "   private static synchronized void assinar(Contrato k) {"),
])

# Investidor.oferecer: the offer to buy the club, the human accepts or refuses
patch("Investidor.java", [
    ("SwingUtilities.invokeLater(() -> oferecer(meu, nome, agora, anual));", "exporter.HumanPolicy.later(() -> oferecerNoHarness(meu, nome, agora, anual));"),
    ("   private static void aceitar(ah meu, String nome, long agora, long anual) {",
     "   private static void oferecerNoHarness(ah meu, String nome, long agora, long anual) {\n"
     "      try {\n"
     "         if (exporter.HumanPolicy.acceptInvestor()) {\n"
     "            aceitar(meu, nome, agora, anual);\n"
     "         }\n"
     "      } finally {\n"
     "         synchronized (Investidor.class) {\n"
     "            oferecendo = false;\n"
     "         }\n"
     "      }\n"
     "   }\n\n"
     "   private static void aceitar(ah meu, String nome, long agora, long anual) {"),
])

# Mercado.mostrar: the rich clubs' offers and the loan requests on the game's offer screen
# (a.jm), the human accepts; an accepted offer rebuilds the main screen (best.S.aY(2)), whose
# card runs the mod's clock again
patch("Mercado.java", [
    ("SwingUtilities.invokeLater(() -> mostrar(propostas, 0));", "exporter.HumanPolicy.later(() -> mostrarNoHarness(propostas));"),
    ("   /** Shows the offers one after another on the game's offer screen. */",
     "   private static void mostrarNoHarness(List<Object[]> propostas) {\n"
     "      for (Object[] x : propostas) {\n"
     "         F p = (F) x[0];\n"
     "         ah comprador = (ah) x[1];\n"
     "         int valor = (int) Math.min(Integer.MAX_VALUE, (Long) x[2]);\n"
     "         boolean emprestimo = x.length > 3 && Boolean.TRUE.equals(x[3]);\n"
     "         if (p.fg() == null || !Economia.humano(p.fg())) {\n"
     "            continue;\n"
     "         }\n"
     "         if (!exporter.HumanPolicy.acceptSquadOffer(emprestimo)) {\n"
     "            continue;\n"
     "         }\n"
     "         if (!emprestimo) {\n"
     "            p.a(comprador, valor, false, false, false);\n"
     "         } else {\n"
     "            ah dono = p.fg();\n"
     "            p.q(comprador);\n"
     "            Emprestimos.aoEmprestar(p, dono, comprador, (long) valor);\n"
     "         }\n"
     "         exporter.HumanPolicy.mainScreenAgain();\n"
     "      }\n"
     "   }\n\n"
     "   /** Shows the offers one after another on the game's offer screen. */"),
])
