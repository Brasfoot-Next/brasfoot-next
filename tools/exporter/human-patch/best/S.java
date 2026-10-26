package best;

import a.eg;
import a.hU;
import f.B;
import java.util.ArrayList;
import javax.swing.JDialog;
import javax.swing.JFrame;
import javax.swing.JPanel;
import javax.swing.JTable;

/**
 * The game's main window best.S for exporter.HumanHarness (headless): no frame, and each dialog it
 * opens goes to exporter.HumanPolicy.dialog, which records it and does what the human of the
 * harness does there (close it without accepting anything).
 */
public class S {
   public S(boolean var1) {
   }

   public static JPanel aY(int var0) {
      exporter.HumanPolicy.dialog("S.aY(" + var0 + ")");
      return null;
   }

   public static hU iE() {
      return null;
   }

   public static eg iF() {
      return null;
   }

   public static JFrame db() {
      return null;
   }

   /** a.dJ */
   public static void a(ah var0, at var1, int var2) {
      exporter.HumanPolicy.dialog("a.dJ");
   }

   /** a.gO: job offers to a human coach */
   public static void a(ArrayList var0, al var1, int var2) {
      exporter.HumanPolicy.dialog("a.gO");
      exporter.HumanPolicy.jobOffers(var0, var1, var2);
   }

   /** a.gE */
   public static void l(al var0) {
      exporter.HumanPolicy.dialog("a.gE");
   }

   /** a.bl */
   public static void K(ArrayList var0) {
      exporter.HumanPolicy.dialog("a.bl");
   }

   /** a.dm */
   public static void a(al var0, al var1, ah var2, int var3) {
      exporter.HumanPolicy.dialog("a.dm");
   }

   /** a.dN */
   public static void cX() {
      exporter.HumanPolicy.dialog("a.dN");
   }

   /** a.hz */
   public static void a(String var0, boolean var1) {
      exporter.HumanPolicy.dialog("a.hz");
   }

   /** a.hF */
   public static void iG() {
      exporter.HumanPolicy.dialog("a.hF");
   }

   /** a.aP */
   public static void a(ah var0, boolean var1) {
      exporter.HumanPolicy.dialog("a.aP");
   }

   /** a.dk */
   public static void a(F var0, JDialog var1) {
      exporter.HumanPolicy.dialog("a.dk");
   }

   /** a.di */
   public static void v(ah var0) {
      exporter.HumanPolicy.dialog("a.di");
   }

   /** a.ay */
   public static void a(int var0, at var1) {
      exporter.HumanPolicy.dialog("a.ay");
   }

   /** a.al */
   public static void b(int var0, at var1) {
      exporter.HumanPolicy.dialog("a.al");
   }

   /** a.dg */
   public static void e(al var0) {
      exporter.HumanPolicy.dialog("a.dg");
   }

   /** a.iu */
   public static void a(B var0, Z var1, int var2) {
      exporter.HumanPolicy.dialog("a.iu");
   }

   /** a.fB */
   public static void a(JTable var0) {
      exporter.HumanPolicy.dialog("a.fB");
   }

   /** a.h */
   public static void w(ah var0) {
      exporter.HumanPolicy.dialog("a.h");
   }

   /** a.d */
   public static void x(ah var0) {
      exporter.HumanPolicy.dialog("a.d");
   }

   /** a.iA */
   public static void a(ah var0, ah var1, boolean var2) {
      exporter.HumanPolicy.dialog("a.iA");
   }

   public static JDialog iH() {
      return null;
   }

   public static boolean iI() {
      return false;
   }
}
