package exporter;

import java.awt.Component;
import java.awt.Container;
import java.awt.Dimension;
import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.image.BufferedImage;
import java.io.File;
import java.lang.reflect.Method;
import java.nio.file.Path;
import java.nio.file.Paths;
import javax.imageio.ImageIO;
import javax.swing.JComponent;

/**
 * Reference pictures of the game's screens for the Rust app: builds the game's own panels, with
 * the mod's look (bfmod.Visual), without a window, and paints them to PNG. Runs headless.
 *
 * Usage: Screens <out dir> new             the new-game screens (a.hU, a.bV), from user.dir's teams
 *        Screens <out dir> career <save>   the career screens of a fictional save
 * user.dir must be a copy of the game folder (tools/exporter/export.sh screens).
 */
public class Screens {
   static Path out;

   static void layout(Component c) {
      if (c instanceof Container k) {
         k.doLayout();
         for (Component x : k.getComponents()) layout(x);
      }
   }

   /** Paints a panel at its preferred size (or the given one) into <name>.png. */
   static void shot(String name, JComponent panel, Dimension size) throws Exception {
      Dimension d = size != null ? size : panel.getPreferredSize();
      panel.setSize(d);
      layout(panel);
      layout(panel);
      BufferedImage img = new BufferedImage(d.width, d.height, BufferedImage.TYPE_INT_RGB);
      Graphics2D g = img.createGraphics();
      g.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON);
      g.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON);
      panel.printAll(g);
      g.dispose();
      File f = out.resolve(name + ".png").toFile();
      ImageIO.write(img, "png", f);
      System.out.println("wrote " + f + " " + d.width + "x" + d.height);
   }

   static void modLook() throws Exception {
      Method m = Class.forName("bfmod.Visual").getDeclaredMethod("instalar");
      m.setAccessible(true);
      m.invoke(null);
   }

   static void newGame() throws Exception {
      new c.a();
      best.p.dd();
      a.hU settings = new a.hU(null);
      shot("hU-configuracoes-iniciais", settings, new Dimension(1024, 728));
      // "Iniciar Jogo >>" with the default league (Brazil)
      c.a.SR.I(2022);
      best.p.dg();
      a.bV coach = new a.bV(null);
      shot("bV-adicionar-tecnico", coach, new Dimension(1024, 728));
   }

   public static void main(String[] args) throws Exception {
      out = Paths.get(args[0]);
      out.toFile().mkdirs();
      modLook();
      // the registration the game keeps in the Java preferences (read only): the full version
      best.h2.mg();
      new best.M(true);
      best.n.cU();
      switch (args[1]) {
         case "new" -> newGame();
         default -> throw new IllegalArgumentException(args[1]);
      }
      System.exit(0);
   }
}
