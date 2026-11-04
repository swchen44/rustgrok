import org.opengrok.indexer.analysis.CompatibleAnalyser;
import org.apache.lucene.analysis.TokenStream;
import org.apache.lucene.analysis.tokenattributes.CharTermAttribute;
import org.apache.lucene.analysis.tokenattributes.TermToBytesRefAttribute;
import org.apache.lucene.analysis.tokenattributes.OffsetAttribute;
public class RustgrokLongTokenProbe {
 public static void main(String[] args) throws Exception {
  String input = "a".repeat(32765) + "\uD801\uDC00";
  try (var analyzer=new CompatibleAnalyser(); TokenStream stream=analyzer.tokenStream("full",input)) {
   var term=stream.addAttribute(CharTermAttribute.class);
   var bytes=stream.addAttribute(TermToBytesRefAttribute.class);
   var offset=stream.addAttribute(OffsetAttribute.class);
   stream.reset();
   while(stream.incrementToken()) {
    var ref=bytes.getBytesRef();
    int n=term.length();
    System.out.printf("units=%d last-unit=%04X unpaired-high=%s start=%d end=%d byte-length=%d bytes-tail=",n,(int)term.charAt(n-1),Character.isHighSurrogate(term.charAt(n-1)),offset.startOffset(),offset.endOffset(),ref.length);
    for(int i=Math.max(0,ref.length-6);i<ref.length;i++)System.out.printf("%02X",ref.bytes[ref.offset+i]&255);
    System.out.println();
   }
   stream.end();
  }
 }
}
