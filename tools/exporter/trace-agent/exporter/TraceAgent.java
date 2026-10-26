package exporter;

import java.lang.instrument.ClassFileTransformer;
import java.lang.instrument.Instrumentation;
import java.security.ProtectionDomain;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import jdk.internal.org.objectweb.asm.ClassReader;
import jdk.internal.org.objectweb.asm.ClassVisitor;
import jdk.internal.org.objectweb.asm.ClassWriter;
import jdk.internal.org.objectweb.asm.MethodVisitor;
import jdk.internal.org.objectweb.asm.Opcodes;

/**
 * Debug agent for the differential tests (BF_TRACE_METHODS): at the entry of each listed method
 * of the game's own classes, prints the counters of java.util.Random (TraceAgent.hit), so a run
 * can be lined up with the Rust side's Runtime::checkpoint labels. The list is
 * "class.method[,class.method...]" (e.g. best.F.gc); the method may carry its descriptor, for
 * overloads (best.F.a(Ljava/lang/String;IZ)Z); a trailing '*' prints every call, else only the
 * first one. The classes stay the original bytecode: only the call is added.
 */
public final class TraceAgent {
   static final Map<String, Set<String>> METHODS = new HashMap<>();
   static final Set<String> EVERY = new HashSet<>();
   static final Set<String> SEEN = new HashSet<>();

   public static void premain(String args, Instrumentation inst) {
      for (String spec : args.split(",")) {
         boolean every = spec.endsWith("*");
         if (every) spec = spec.substring(0, spec.length() - 1);
         int paren = spec.indexOf('(');
         int dot = spec.lastIndexOf('.', paren < 0 ? spec.length() : paren);
         String cls = spec.substring(0, dot).replace('.', '/');
         String method = spec.substring(dot + 1);
         METHODS.computeIfAbsent(cls, k -> new HashSet<>()).add(method);
         if (every) EVERY.add(cls.replace('/', '.') + "." + method);
      }
      inst.addTransformer(new ClassFileTransformer() {
         @Override
         public byte[] transform(Module module, ClassLoader loader, String name, Class<?> redefined, ProtectionDomain domain, byte[] bytes) {
            Set<String> wanted = METHODS.get(name);
            if (wanted == null) return null;
            ClassReader reader = new ClassReader(bytes);
            ClassWriter writer = new ClassWriter(reader, 0);
            reader.accept(new ClassVisitor(Opcodes.ASM9, writer) {
               @Override
               public MethodVisitor visitMethod(int access, String method, String desc, String sig, String[] exceptions) {
                  MethodVisitor mv = super.visitMethod(access, method, desc, sig, exceptions);
                  String label;
                  if (wanted.contains(method + desc)) label = name.replace('/', '.') + "." + method + desc;
                  else if (wanted.contains(method)) label = name.replace('/', '.') + "." + method;
                  else return mv;
                  String hitLabel = label;
                  return new MethodVisitor(Opcodes.ASM9, mv) {
                     @Override
                     public void visitCode() {
                        super.visitCode();
                        super.visitLdcInsn(hitLabel);
                        super.visitMethodInsn(Opcodes.INVOKESTATIC, "exporter/TraceAgent", "hit", "(Ljava/lang/String;)V", false);
                     }

                     @Override
                     public void visitMaxs(int stack, int locals) {
                        super.visitMaxs(Math.max(stack, 1), locals);
                     }
                  };
               }
            }, 0);
            return writer.toByteArray();
         }
      });
   }

   /** The label (class.method, the game's obfuscated name) and the counters, as HumanHarness.checkpoint prints them. */
   public static synchronized void hit(String label) {
      if (!EVERY.contains(label) && !SEEN.add(label)) return;
      try {
         java.lang.reflect.Field f = java.util.Random.class.getDeclaredField("bfNext");
         f.setAccessible(true);
         System.out.println("rng " + label + " " + f.getLong(null) + " draws " + java.util.Random.class.getField("bfDraws").getLong(null));
      } catch (Exception e) {
         throw new RuntimeException(e);
      }
   }
}
