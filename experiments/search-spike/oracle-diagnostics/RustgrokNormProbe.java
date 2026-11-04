import org.apache.lucene.util.SmallFloat;
public class RustgrokNormProbe {
 public static void main(String[] args) {
  for (int i=0;i<256;i++) System.out.println(i+" "+SmallFloat.byte4ToInt((byte)i));
 }
}
