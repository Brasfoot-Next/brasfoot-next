package a;

import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;
import java.awt.Frame;
import java.awt.Point;
import java.awt.Rectangle;
import java.io.File;
import java.io.IOException;
import java.util.ArrayList;
import java.util.Collections;
import javax.sound.sampled.AudioInputStream;
import javax.sound.sampled.AudioSystem;
import javax.sound.sampled.Clip;
import javax.sound.sampled.LineUnavailableException;
import javax.sound.sampled.UnsupportedAudioFileException;
import javax.swing.ActionMap;
import javax.swing.BorderFactory;
import javax.swing.GroupLayout;
import javax.swing.ImageIcon;
import javax.swing.InputMap;
import javax.swing.JComponent;
import javax.swing.JDialog;
import javax.swing.JFrame;
import javax.swing.JLabel;
import javax.swing.JPanel;
import javax.swing.JProgressBar;
import javax.swing.JScrollPane;
import javax.swing.JTable;
import javax.swing.JViewport;
import javax.swing.KeyStroke;
import javax.swing.Timer;
import javax.swing.GroupLayout.Alignment;
import javax.swing.LayoutStyle.ComponentPlacement;

// Copy of the game's a.dV for exporter.HumanHarness (headless): no timer, and each dialog replaced by
// exporter.HumanPolicy, which runs the dialog's own class without a window.
public class dV extends JPanel {
   private static JFrame Br;
   private static dV GS;
   private static ArrayList vK = new ArrayList();
   private static int GT = 0;
   public Timer GO = null;
   private static int d = 1;
   private static int dB = 1;
   private boolean GU = false;
   private int GV = -1;
   private int GW = -1;
   private Color GX = new Color(51, 102, 0);
   private static ArrayList zD = new ArrayList();
   private static ArrayList zE = new ArrayList();
   private static ArrayList GY = new ArrayList();
   private static boolean GZ = true;
   private int Ha = 720;
   private static best.at Hb = null;
   private static int Hc = -1;
   private AudioInputStream[] Hd = new AudioInputStream[7];
   private static Clip[] He = new Clip[7];
   private JLabel uh;
   private JProgressBar Hf;
   private JScrollPane ut;
   private JLabel Hg;
   private JLabel xI;
   private JLabel Hh;
   private JLabel Hi;
   private JLabel Hj;
   private JLabel Hk;
   private JLabel Hl;
   private JLabel Hm;
   private JLabel Hn;
   private JLabel Ho;
   private JLabel Hp;
   private JPanel Hq;
   private JPanel Hr;
   private JTable Hs;
   private JLabel zM;
   private JLabel Ht;

   public dV(JFrame var1, int var2) {
      Br = var1;
      GS = this;
      this.Ha = var2;
      this.mJ();
      this.Hf.setStringPainted(false);
      this.Hf.setForeground(this.GX);
      this.Hf.setBorder(BorderFactory.createLineBorder(Color.WHITE));
      this.Hf.setMaximum(45);
      this.nc();
      this.mY();
   }

   private void mY() {
      try {
         this.a(this.Hr);
         this.a(this.ut);
         this.a(this.Hs);
      } catch (Exception var2) {
      }
   }

   private void a(JComponent var1) {
      InputMap var2 = var1.getInputMap(1);
      var1.getInputMap().put(KeyStroke.getKeyStroke(112, 0), "f1");
      var1.getInputMap().put(KeyStroke.getKeyStroke(113, 0), "f2");
      var1.getInputMap().put(KeyStroke.getKeyStroke(114, 0), "f3");
      var1.getInputMap().put(KeyStroke.getKeyStroke(115, 0), "f4");
      var1.getInputMap().put(KeyStroke.getKeyStroke(116, 0), "f5");
      if (c.a.vL()) {
         var1.getInputMap().put(KeyStroke.getKeyStroke(116, 0), "f6");
      }

      var1.getInputMap().put(KeyStroke.getKeyStroke(10, 0), "st");
      var1.getInputMap().put(KeyStroke.getKeyStroke(32, 0), "st");
      ActionMap var3 = this.getActionMap();
      var1.getActionMap().put("st", new dW(this));
      var1.getActionMap().put("f1", new dY(this));
      var1.getActionMap().put("f2", new dZ(this));
      var1.getActionMap().put("f3", new ea(this));
      var1.getActionMap().put("f4", new eb(this));
      var1.getActionMap().put("f5", new ec(this));
      var1.getActionMap().put("f6", new ed(this));
   }

   private void setVelocidade(int var1) {
      if (var1 < best.aq.rz.length) {
         d = best.aq.rz[var1];
      }
   }

   public void Y(ArrayList var1) {
      GT = 0;
      vK.clear();
      dB = 1;
      this.GU = false;
      GZ = true;

      for (int var2 = 0; var2 < var1.size(); var2++) {
         boolean var3 = true;

         for (int var4 = 0; var4 < c.a.SR.S().size(); var4++) {
            if (((best.I)c.a.SR.S().get(var4)).hy() == var1.get(var2)) {
               if (var3) {
                  if (var2 == 0) {
                     this.a(((best.I)c.a.SR.S().get(var4)).hy(), ((best.I)c.a.SR.S().get(var4)).ht());
                  }

                  if (var2 != -1) {
                     components.v var5 = new components.v();
                     var5.L(this.a(((best.I)c.a.SR.S().get(var4)).hy(), (best.I)c.a.SR.S().get(var4), ((best.I)c.a.SR.S().get(var4)).ht()));
                     vK.add(var5);
                  }

                  var3 = false;
               }

               components.v var13 = new components.v();
               var13.n((best.I)c.a.SR.S().get(var4));
               var13.tW();
               if (var13.tR().hc() != null) {
                  var13.tR().hc().Q(var13.tR().hp());
                  var13.tR().hc().O(var13.tR().hp());
               }

               if (var13.tR().hd() != null) {
                  var13.tR().hd().Q(var13.tR().hq());
                  var13.tR().hd().O(var13.tR().hq());
               }

               if (((best.I)c.a.SR.S().get(var4)).hy().b() == 7) {
                  Object var6 = null;
                  var13.p(var13.tR().ik());
               } else if (var13.tR().ev() == null) {
                  if (((best.I)c.a.SR.S().get(var4)).hy().b() == 4) {
                     if (((best.I)c.a.SR.S().get(var4)).hy() instanceof f.v) {
                        var13.p(f.v.yD());
                     } else if (((best.I)c.a.SR.S().get(var4)).hy() instanceof f.r) {
                        var13.p(f.r.yD());
                     }
                  } else if (((best.I)c.a.SR.S().get(var4)).hy().b() == 6) {
                     if (((best.I)c.a.SR.S().get(var4)).hy() instanceof f.H) {
                        var13.p(f.H.yD());
                     } else if (((best.I)c.a.SR.S().get(var4)).hy() instanceof f.x) {
                        var13.p(f.x.yD());
                     }
                  }
               }

               vK.add(var13);
            }
         }
      }

      boolean var8 = false;
      this.GW = -1;
      int var9 = 0;

      for (int var10 = 0; var10 < vK.size(); var10++) {
         if (((components.v)vK.get(var10)).ei()) {
            var8 = true;
            this.GW = var10;
            break;
         }
      }

      for (int var11 = 0; var11 < vK.size(); var11++) {
         if (((components.v)vK.get(var11)).ei()) {
            var9++;
         }
      }

      if (var8 && this.GW >= 0) {
         this.pZ();
      } else {
         this.Hr.setVisible(false);
      }

      int var12 = c.a.SR.getVelocidade();
      if (!var8) {
         var12 = c.a.SR.getVelocidadeNH();
      }

      if (var8) {
         d = best.aq.rz[var12];
      } else {
         d = best.aq.rA[var12];
      }

      if (c.a.SR.bD()) {
         d = 1;
      }

      GS.Hs.addNotify();
      if (this.GW >= 20) {
         JViewport var14 = (JViewport)GS.Hs.getParent();
         Rectangle var15 = GS.Hs.getCellRect(this.GW + 8, 0, true);
         Point var7 = var14.getViewPosition();
         var15.setLocation(var15.x - var7.x, var15.y - var7.y);
         GS.Hs.scrollRectToVisible(var15);
      }

      Hc = this.GW;
      this.qf();
      this.oR();
   }

   public void pZ() {
      this.Hr.setVisible(true);
      this.Hp.setBackground(((components.v)vK.get(this.GW)).tR().hc().kB());
      this.Hp.setForeground(((components.v)vK.get(this.GW)).tR().hc().kC());
      this.Hp.setText(((components.v)vK.get(this.GW)).tR().hc().getNome() + " ");
      this.Ho.setBackground(((components.v)vK.get(this.GW)).tR().hd().kB());
      this.Ho.setForeground(((components.v)vK.get(this.GW)).tR().hd().kC());
      this.Ho.setText(" " + ((components.v)vK.get(this.GW)).tR().hd().getNome());
      this.Hh.setIcon(((components.v)vK.get(this.GW)).tR().hc().kP());
      this.Hi.setIcon(((components.v)vK.get(this.GW)).tR().hd().kP());
      if (((components.v)vK.get(this.GW)).tR().ev() != null) {
         this.Hl.setText(((components.v)vK.get(this.GW)).tR().ev().dS());
         this.Hm.setText(Integer.toString(((components.v)vK.get(this.GW)).hU()));
      } else if (((components.v)vK.get(this.GW)).ik() != null) {
         this.Hl.setText(((components.v)vK.get(this.GW)).ik());
         this.Hm.setText("");
      } else {
         this.Hl.setText("");
         this.Hm.setText("");
      }

      this.Hj.setText("");
      this.Hk.setText("");
      this.Hj.setIcon(null);
      this.Hk.setIcon(null);
      this.Hn.setText("0 x 0");
      this.xI.setText("");
      if (((components.v)vK.get(this.GW)).tR().hy() != null) {
         this.Ht.setText(this.a(((components.v)vK.get(this.GW)).tR().hy(), ((components.v)vK.get(this.GW)).tR(), ((components.v)vK.get(this.GW)).tR().ht()));
      } else {
         this.Ht.setText("");
      }
   }

   private String a(best.at var1, best.I var2, best.L var3) {
      String var4 = "";
      if (var1.b() == 3 && c.a.SR.isVerEstaduaisAgrupados()) {
         var4 = best.aq.rZ[var1.el()] + " - " + var1.is() + " - " + var2.hJ()[1];
      } else if (var1.b() == 3) {
         var4 = var1.is() + " - " + var2.hJ()[1];
      } else if (var1.b() == 1) {
         var4 = var1.is() + " - " + var2.hJ()[1];
         if (var3 != null && var3 instanceof f.z) {
            if (((f.z)var3).zf() == 1099) {
               var4 = "Mata Mata Ascenso - " + var2.hJ()[1];
            } else if (((f.z)var3).zf() == 1098) {
               var4 = "Playoff rebaixamento - " + var2.hJ()[1];
            }
         }
      } else if (var1.b() == 10) {
         var4 = var1.getNome() + " - " + var2.hJ()[1];
      } else if (var1.b() == 7 && var1.el() != 7) {
         var4 = var1.getNome() + " - " + var2.hJ()[1];
      } else if (var1.b() == 9) {
         if (var1.el() < best.aq.sa.length) {
            var4 = best.aq.sa[var1.el()] + " - " + var2.hJ()[1];
         } else if (var1.el() == 70) {
            var4 = "Classificatório Eurocopa - " + var2.hJ()[1];
         }

         if (var3 != null && var3 instanceof f.s && ((f.s)var3).ze() == 7701) {
            var4 = "Torneio Repescagem";
         }
      } else if (var1 instanceof f.I) {
         var4 = "Supercopa " + best.ad.bt(((f.I)var1).getPais());
      } else if (var1.b() == 8) {
         var4 = var1.getNome();
      } else {
         var4 = var2.hJ()[1];
      }

      return var4;
   }

   private void a(best.at var1, best.L var2) {
      String var3 = "";
      if (var1 != null) {
         if (var1.b() == 3) {
            if (c.a.SR.isVerEstaduaisAgrupados()) {
               var3 = "Campeonatos estaduais";
            } else {
               var3 = "Campeonato " + best.aq.rZ[var1.el()];
            }
         } else if (var1.b() == 1) {
            if (var1.iq() != null) {
               var3 = "Campeonato " + best.ad.bs(var1.iq().jc());
            }
         } else if (var1.b() == 10) {
            var3 = "Regional";
         } else if (var1.b() == 7 && var1.el() != 7) {
            var3 = "Copas de Seleções";
         } else if (var1.b() == 7 && var1.el() == 7) {
            var3 = "Copas do Mundo";
         } else if (var1.b() == 11) {
            var3 = "Supercopa Nacional";
         } else if (var1.b() == 8) {
            var3 = "Recopa";
         } else if (var1.b() == 9) {
            var3 = "Eliminatórias";
            if (var2 != null && var2 instanceof f.s && ((f.s)var2).ze() == 7701) {
               var3 = "Eliminatórias - Repescagem";
            }
         } else {
            var3 = var1.getNome();
         }
      }

      this.Hg.setText(var3);
   }

   public void oR() {
      // harness: no Swing timer, exporter.HumanHarness calls qa() in a loop
   }

   public void qa() {
      boolean var1 = false;
      GT++;
      if (dB == 1) {
         if (GT > 48) {
            this.qg();
         }
      } else if (dB == 2 && GT >= 51) {
         var1 = true;
      }

      if (qj() < 45) {
         this.uh.setText(Integer.toString(qj()) + "'" + " - " + Integer.toString(dB) + "º tempo");
         this.Hf.setValue(qj());
      } else {
         this.uh.setText("Acréscimos");
      }

      if (var1) {
         this.uh.setText("Fim de jogo");
         this.qc();
      } else {
         this.aa(false);
      }
   }

   public static void qb() {
      try {
         for (int var0 = 0; var0 < He.length; var0++) {
            if (He[var0] != null) {
               He[var0].close();
               He[var0] = null;
            }
         }
      } catch (Exception var1) {
      }
   }

   private void qc() {
      if (!this.GU) {
         this.GU = true;
         this.dk(1);
         this.qe();
         this.qd();
         if (GY.size() > 0) {
            for (int var1 = 0; var1 < GY.size(); var1++) {
               this.g((best.I)GY.get(var1));
            }
         }

         best.s.dJ();
      }
   }

   private void g(best.I var1) {
      if (!var1.hc().jZ() && !var1.hd().jZ()) {
         ArrayList var5 = new ArrayList();
         var5.addAll(var1.hp());
         Collections.sort(var5, h.a.abg);
         zD = var5;
         ArrayList var3 = new ArrayList();
         var3.addAll(var1.hq());
         Collections.sort(var3, h.a.abg);
         zE = var3;
      } else {
         if (var1.hc().jZ()) {
            this.a(var1, var1.hc());
         } else {
            ArrayList var2 = new ArrayList();
            var2.addAll(var1.hp());
            Collections.sort(var2, h.a.abg);
            zD = var2;
         }

         if (var1.hd().jZ()) {
            this.a(var1, var1.hd());
         } else {
            ArrayList var4 = new ArrayList();
            var4.addAll(var1.hq());
            Collections.sort(var4, h.a.abg);
            zE = var4;
         }
      }

      exporter.HumanPolicy.shootout(var1);
   }

   private void a(best.I var1, best.ah var2) {
      exporter.HumanPolicy.shootoutOrder(var1, var2);
   }

   private void qd() {
      GY.clear();

      for (int var1 = 0; var1 < vK.size(); var1++) {
         if (((components.v)vK.get(var1)).tR() != null && ((components.v)vK.get(var1)).tR().hS() && ((components.v)vK.get(var1)).tR().hk()) {
            if (!((components.v)vK.get(var1)).tR().hc().jZ() && !((components.v)vK.get(var1)).tR().hd().jZ()) {
               if (c.a.SR.getVerDecisaoPenNaoHumano() == 1) {
                  GY.add(((components.v)vK.get(var1)).tR());
               }
            } else {
               GY.add(((components.v)vK.get(var1)).tR());
            }
         }
      }
   }

   public void qe() {
      for (int var1 = 0; var1 < vK.size(); var1++) {
         boolean var2 = false;
         if (((components.v)vK.get(var1)).tR() != null && ((components.v)vK.get(var1)).tR().hu() != ((components.v)vK.get(var1)).tT()) {
            var2 = true;
         }

         if (var2) {
            ((components.v)vK.get(var1)).tR().hF();
         }
      }
   }

   public File dj(int var1) {
      return c.a.dj(var1);
   }

   private void qf() {
      if (c.a.SR.isUsaSons()) {
         for (int var1 = 0; var1 < this.Hd.length; var1++) {
            File var2 = this.dj(var1);
            if (var2 != null) {
               try {
                  this.Hd[var1] = AudioSystem.getAudioInputStream(var2);
               } catch (Exception var4) {
               }
            }
         }
      }
   }

   public void dk(int var1) {
      if (c.a.SR.isUsaSons()) {
         Object var2 = null;
         if (var1 < this.Hd.length && this.Hd[var1] != null) {
            if (He[var1] == null) {
               try {
                  He[var1] = AudioSystem.getClip();
                  He[var1].open(this.Hd[var1]);
               } catch (Exception var5) {
                  var5.printStackTrace();
               }
            }

            if (He[var1] != null) {
               try {
                  He[var1].stop();
                  He[var1].setFramePosition(0);
                  He[var1].start();
               } catch (Exception var4) {
               }
            }
         }
      }
   }

   public void dl(int var1) {
      if (c.a.SR.isUsaSons()) {
         AudioInputStream var2 = null;
         Clip var3 = null;

         try {
            File var4 = this.dj(var1);
            if (var4 != null) {
               var2 = AudioSystem.getAudioInputStream(var4);
            }
         } catch (UnsupportedAudioFileException var7) {
            var7.printStackTrace();
         } catch (IOException var8) {
            var8.printStackTrace();
         }

         try {
            if (var2 != null) {
               var3 = AudioSystem.getClip();
               var3.open(var2);
               var3.start();
            }
         } catch (LineUnavailableException var5) {
            var5.printStackTrace();
         } catch (IOException var6) {
            var6.printStackTrace();
         }
      }
   }

   public boolean dm(int var1) {
      if (!c.a.SR.isUsaSons()) {
         return false;
      } else {
         AudioInputStream var2 = null;

         try {
            File var3 = this.dj(var1);
            if (var3 != null) {
               var2 = AudioSystem.getAudioInputStream(var3);
            }
         } catch (UnsupportedAudioFileException var6) {
            var6.printStackTrace();
         } catch (IOException var7) {
            var7.printStackTrace();
         }

         try {
            if (var2 != null) {
               Clip var8 = AudioSystem.getClip();
               var8.open(var2);
               var8.start();
               var8.close();
            }
         } catch (LineUnavailableException var4) {
            var4.printStackTrace();
         } catch (IOException var5) {
            var5.printStackTrace();
         }

         return true;
      }
   }

   public void qg() {
      GT = 0;
      dB = 2;
      if (c.a.SR.getVerJanelaSubs() == 1) {
         this.dk(0);
         this.uh.setText("Intervalo");

         for (int var1 = 0; var1 < vK.size(); var1++) {
            if (((components.v)vK.get(var1)).ei()) {
               this.e(var1, true);
            }
         }
      }

      for (int var2 = 0; var2 < vK.size(); var2++) {
         if (((components.v)vK.get(var2)).ei()) {
            ((components.v)vK.get(var2)).tR().m(2, 0);
         }
      }

      this.Hj.setText("");
      this.Hj.setIcon(null);
      this.Hk.setIcon(null);
      this.Hk.setText("");
   }

   public void aa(boolean var1) {
      if (!var1) {
         for (int var2 = 0; var2 < vK.size(); var2++) {
            if (((components.v)vK.get(var2)).ei()) {
               int[] var3 = ((components.v)vK.get(var2)).tR().ib();
               if (var3[dB] <= GT) {
                  ((components.v)vK.get(var2)).tR().q(GT, dB);
               }

               if (var2 == this.GW) {
                  this.xI.setText(((components.v)vK.get(var2)).tR().ha());
               }
            }
         }
      }

      for (int var4 = 0; var4 < vK.size(); var4++) {
         if (((components.v)vK.get(var4)).tR() != null) {
            for (int var5 = 0; var5 < ((components.v)vK.get(var4)).tR().hE().size(); var5++) {
               if (!((best.A)((components.v)vK.get(var4)).tR().hE().get(var5)).isDone()
                  && ((best.A)((components.v)vK.get(var4)).tR().hE().get(var5)).en() == dB
                  && ((best.A)((components.v)vK.get(var4)).tR().hE().get(var5)).em() <= GT) {
                  this.a((components.v)vK.get(var4), ((components.v)vK.get(var4)).tR(), (best.A)((components.v)vK.get(var4)).tR().hE().get(var5), var4);
               }
            }
         }
      }

      if (GS.GV >= 0) {
         GZ = false;
         GS.e(GS.GV, false);
         GZ = true;
         GS.GV = -1;
      }

      GS.Hs.addNotify();
      if (GZ) {
         GS.oR();
      }
   }

   public void a(components.v var1, best.I var2, best.A var3, int var4) {
      if (var3.b() == 8) {
         var3.p(true);
      }

      if (!var3.isDone()) {
         var3.p(true);
         byte var5 = 0;
         if (var3.cu() == var2.hc()) {
            var5 = 1;
         } else if (var3.cu() == var2.hd()) {
            var5 = 2;
         }

         if (var5 > 0) {
            var1.a(var3);
            if (var3.b() == 1) {
               if (var3.el() == 3 && var1.ei()) {
                  this.dk(4);
                  if (!var3.cu().jZ()) {
                     this.b(var1, var3);
                  } else {
                     this.a(var1, var3);
                  }

                  var3.q(true);
               } else {
                  var1.dJ(var5);
                  if (var1.ei()) {
                     if (var3.cu().jZ()) {
                        this.dk(2);
                     } else {
                        this.dk(3);
                     }
                  }
               }
            } else if (var3.b() != 3 && var3.b() != 4) {
               if (var3.b() == 5) {
                  if (var1.ei()) {
                     this.dk(6);
                  }

                  if (var3.cu().jZ()) {
                     GS.e(var4, false);
                  }
               }
            } else {
               if (var1.ei()) {
                  this.dk(5);
               }

               if (var3.cu().jZ()) {
                  GS.e(var4, false);
               }
            }
         }

         GS.Hs.addNotify();
         if (this.Hr.isVisible() && this.GW >= 0 && this.GW == var4) {
            this.Hn.setText(var1.tT() + " x " + var1.tU());
            if (var5 == 1) {
               this.Hj.setText(var1.tV().er());
               this.Hj.setIcon(var1.tV().es());
            } else {
               this.Hk.setText(var1.tV().er());
               this.Hk.setIcon(var1.tV().es());
            }
         }
      }
   }

   private void a(components.v var1, best.A var2) {
      exporter.HumanPolicy.chooseTaker(var1, var2);
      this.b(var1, var2);
   }

   private void b(components.v var1, best.A var2) {
      exporter.HumanPolicy.penalty(var1, var2, this);
   }

   public String h(best.I var1) {
      String var2 = null;
      int var3 = -1;
      var3 = var1.hy().b();
      if (var3 == 3) {
         var2 = var1.hy().getNome() + " " + var1.hy().ip();
      }

      return var2;
   }

   public void nc() {
      b.M var1 = new b.M(this);
      this.Hs.setModel(var1);
      int[] var2 = new int[]{250, 200, 40, 40, 200, 300};

      for (int var3 = 0; var3 < var2.length; var3++) {
         this.Hs.getColumnModel().getColumn(var3).setPreferredWidth(var2[var3]);
      }

      this.Hs.setAutoResizeMode(3);
      this.Hs.setRowHeight(25);
      this.Hs.setTableHeader(null);
      this.Hs.setShowGrid(false);
      this.Hs.setDefaultRenderer(components.v.class, new b.aG());
      this.Hs.setIntercellSpacing(new Dimension(5, 5));
      this.Hs.setAutoCreateRowSorter(false);
      this.Hs.setCellSelectionEnabled(false);
      this.Hs.setRowSelectionAllowed(false);
      this.Hs.setBackground(best.aq.ot);
      this.Hs.setSelectionBackground(new Color(51, 102, 0));
      this.Hs.setFillsViewportHeight(true);
      this.mH();
   }

   private void qh() {
      if (Hc >= 0) {
         this.GV = Hc;
      }
   }

   private void mH() {
      this.Hs.addMouseListener(new ef(this));
      dX var1 = new dX(this);
      this.Hn.addMouseListener(var1);
      this.Ho.addMouseListener(var1);
      this.Hp.addMouseListener(var1);
      this.Hl.addMouseListener(var1);
      this.Hm.addMouseListener(var1);
      this.xI.addMouseListener(var1);
      this.Hh.addMouseListener(var1);
      this.Hi.addMouseListener(var1);
   }

   private void e(int var1, boolean var2) {
      exporter.HumanPolicy.substitutions((components.v)vK.get(var1), var2);
   }

   public ArrayList qi() {
      return vK;
   }

   public static int qj() {
      return GT;
   }

   public static int en() {
      return dB;
   }

   public static ArrayList qk() {
      return zD;
   }

   public static ArrayList ql() {
      return zE;
   }

   public static void a(int var0, ArrayList var1) {
      if (var0 == 1) {
         zD = var1;
      } else {
         zE = var1;
      }
   }

   public static best.at qm() {
      return Hb;
   }

   public static void v(best.at var0) {
      Hb = var0;
   }

   public static boolean qn() {
      return GZ;
   }

   public static void ab(boolean var0) {
      GZ = var0;
   }

   private void mJ() {
      this.Hq = new JPanel();
      this.Hg = new JLabel();
      this.Hf = new JProgressBar();
      this.uh = new JLabel();
      this.Hr = new JPanel();
      this.xI = new JLabel();
      this.Hn = new JLabel();
      this.Hh = new JLabel();
      this.Hi = new JLabel();
      this.Ho = new JLabel();
      this.Hp = new JLabel();
      this.Hk = new JLabel();
      this.Hm = new JLabel();
      this.Hj = new JLabel();
      this.Hl = new JLabel();
      this.Ht = new JLabel();
      this.zM = new JLabel();
      this.ut = new JScrollPane();
      this.Hs = new JTable();
      this.setBackground(best.aq.ou);
      this.setPreferredSize(new Dimension(1024, 710));
      this.Hq.setBackground(best.aq.ou);
      this.zM.setBackground(best.aq.ou);
      this.zM.setOpaque(true);
      this.Hq.setOpaque(true);
      this.Hq.setPreferredSize(new Dimension(1024, 40));
      this.Hg.setFont(new Font("Tahoma", 1, 14));
      this.Hg.setForeground(new Color(255, 255, 153));
      this.Hg.setText("Informação da rodada");
      this.uh.setFont(new Font("Tahoma", 1, 12));
      this.uh.setForeground(new Color(255, 255, 255));
      this.uh.setText("Tempo");
      GroupLayout var1 = new GroupLayout(this.Hq);
      this.Hq.setLayout(var1);
      var1.setHorizontalGroup(
         var1.createParallelGroup(Alignment.LEADING)
            .addGroup(
               var1.createSequentialGroup()
                  .addContainerGap()
                  .addComponent(this.Hg, -2, 566, -2)
                  .addPreferredGap(ComponentPlacement.RELATED, 92, 32767)
                  .addComponent(this.Hf, -2, 115, -2)
                  .addPreferredGap(ComponentPlacement.UNRELATED)
                  .addComponent(this.uh, -2, 160, -2)
                  .addGap(71, 71, 71)
            )
      );
      var1.setVerticalGroup(
         var1.createParallelGroup(Alignment.LEADING)
            .addGroup(
               Alignment.TRAILING,
               var1.createSequentialGroup()
                  .addContainerGap()
                  .addGroup(
                     var1.createParallelGroup(Alignment.TRAILING)
                        .addComponent(this.Hf, Alignment.LEADING, -1, -1, 32767)
                        .addGroup(
                           Alignment.LEADING,
                           var1.createSequentialGroup()
                              .addGroup(
                                 var1.createParallelGroup(Alignment.TRAILING).addComponent(this.Hg, Alignment.LEADING).addComponent(this.uh, Alignment.LEADING)
                              )
                              .addGap(0, 3, 32767)
                        )
                  )
                  .addContainerGap()
            )
      );
      this.Hr.setBackground(new Color(0, 0, 0));
      this.Hr.setPreferredSize(new Dimension(1024, 40));
      this.Hr.setLayout(new components.b());
      this.xI.setForeground(new Color(255, 255, 255));
      this.xI.setHorizontalAlignment(0);
      this.xI.setText("Posse de bola");
      this.Hr.add(this.xI, new components.a(800, 4, 190, 120));
      this.Hn.setFont(new Font("Tahoma", 1, 24));
      this.Hn.setForeground(new Color(255, 255, 255));
      this.Hn.setHorizontalAlignment(0);
      this.Hn.setText("3 x  1");
      this.Hr.add(this.Hn, new components.a(430, 40, 100, -1));
      this.Hh.setIcon(new ImageIcon(this.getClass().getResource("/aicons/escudo.png")));
      this.Hr.add(this.Hh, new components.a(170, 20, -1, -1));
      this.Hi.setIcon(new ImageIcon(this.getClass().getResource("/aicons/escudo.png")));
      this.Hr.add(this.Hi, new components.a(725, 20, -1, -1));
      this.Ho.setFont(new Font("Tahoma", 1, 12));
      this.Ho.setForeground(new Color(255, 255, 255));
      this.Ho.setText("Tempo");
      this.Ho.setOpaque(true);
      this.Hr.add(this.Ho, new components.a(537, 40, 180, 30));
      this.Hp.setBackground(new Color(153, 153, 0));
      this.Hp.setFont(new Font("Tahoma", 1, 12));
      this.Hp.setForeground(new Color(255, 255, 255));
      this.Hp.setHorizontalAlignment(4);
      this.Hp.setText("Informação da rodada");
      this.Hp.setOpaque(true);
      this.Hr.add(this.Hp, new components.a(235, 40, 190, 30));
      this.Hk.setFont(new Font("Tahoma", 0, 12));
      this.Hk.setForeground(new Color(255, 255, 204));
      this.Hk.setHorizontalAlignment(2);
      this.Hr.add(this.Hk, new components.a(530, 90, 280, 30));
      this.Hm.setFont(new Font("Tahoma", 1, 12));
      this.Hm.setForeground(new Color(255, 255, 255));
      this.Hm.setHorizontalAlignment(4);
      this.Hm.setText("Informação da rodada");
      this.Hr.add(this.Hm, new components.a(10, 60, 150, -1));
      this.Hj.setFont(new Font("Tahoma", 0, 12));
      this.Hj.setForeground(new Color(255, 255, 204));
      this.Hj.setHorizontalAlignment(4);
      this.Hr.add(this.Hj, new components.a(110, 90, 300, 30));
      this.Hl.setFont(new Font("Tahoma", 1, 12));
      this.Hl.setForeground(new Color(255, 255, 255));
      this.Hl.setHorizontalAlignment(4);
      this.Hl.setText("Informação da rodada");
      this.Hr.add(this.Hl, new components.a(10, 40, 150, -1));
      this.Ht.setForeground(new Color(255, 255, 255));
      this.Ht.setHorizontalAlignment(0);
      this.Ht.setText("semifinal");
      this.Hr.add(this.Ht, new components.a(240, 10, 480, -1));
      this.zM.setIcon(new ImageIcon(this.getClass().getResource("/aiconsj/panelh.png")));
      this.zM.setToolTipText("");
      this.Hr.add(this.zM, new components.a(0, 0, 1008, 130));
      this.ut.setBackground(new Color(0, 51, 0));
      this.ut.setBorder(BorderFactory.createCompoundBorder());
      this.ut.setViewportView(this.Hs);
      GroupLayout var2 = new GroupLayout(this);
      this.setLayout(var2);
      var2.setHorizontalGroup(
         var2.createParallelGroup(Alignment.LEADING)
            .addComponent(this.Hq, -2, -1, -2)
            .addGroup(
               var2.createSequentialGroup()
                  .addGap(10, 10, 10)
                  .addGroup(
                     var2.createParallelGroup(Alignment.LEADING)
                        .addComponent(this.Hr, -2, 1008, -2)
                        .addGroup(var2.createSequentialGroup().addComponent(this.ut).addGap(3, 3, 3))
                  )
                  .addGap(10, 10, 10)
            )
      );
      var2.setVerticalGroup(
         var2.createParallelGroup(Alignment.LEADING)
            .addGroup(
               var2.createSequentialGroup()
                  .addComponent(this.Hq, -2, -1, -2)
                  .addGap(0, 0, 0)
                  .addComponent(this.Hr, -2, 130, -2)
                  .addGap(20, 20, 20)
                  .addComponent(this.ut, -1, 438, 32767)
                  .addGap(20, 20, 20)
            )
      );
   }
}
