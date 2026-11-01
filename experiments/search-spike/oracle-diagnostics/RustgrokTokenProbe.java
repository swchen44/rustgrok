import org.opengrok.indexer.analysis.CompatibleAnalyser;
import org.apache.lucene.analysis.TokenStream;
import org.apache.lucene.analysis.tokenattributes.CharTermAttribute;
import org.apache.lucene.analysis.tokenattributes.OffsetAttribute;
import org.apache.lucene.analysis.tokenattributes.PositionIncrementAttribute;
public class RustgrokTokenProbe {
 public static void main(String[] args) throws Exception {
  try(var analyzer=new CompatibleAnalyser(); TokenStream s=analyzer.tokenStream("full", args[0])) {
   var t=s.addAttribute(CharTermAttribute.class); var o=s.addAttribute(OffsetAttribute.class); var pos=s.addAttribute(PositionIncrementAttribute.class);
   s.reset(); int position=-1; while(s.incrementToken()){position+=pos.getPositionIncrement();System.out.println(position+" "+o.startOffset()+"-"+o.endOffset()+" "+t);}s.end();
  }
 }
}
