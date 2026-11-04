// Independent diagnostic wrapper; OpenGrok and Lucene remain separate oracle dependencies.
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import org.opengrok.indexer.analysis.CompatibleAnalyser;
import org.apache.lucene.analysis.TokenStream;
import org.apache.lucene.analysis.tokenattributes.CharTermAttribute;
import org.apache.lucene.analysis.tokenattributes.OffsetAttribute;
import org.apache.lucene.analysis.tokenattributes.PositionIncrementAttribute;

public class PathTokenOracle {
    public static void main(String[] args) throws Exception {
        String input = new String(Base64.getDecoder().decode(args[0]), StandardCharsets.UTF_8);
        try (CompatibleAnalyser analyzer = new CompatibleAnalyser();
             TokenStream stream = analyzer.tokenStream("path", input)) {
            CharTermAttribute term = stream.addAttribute(CharTermAttribute.class);
            OffsetAttribute offsets = stream.addAttribute(OffsetAttribute.class);
            PositionIncrementAttribute increment = stream.addAttribute(PositionIncrementAttribute.class);
            stream.reset();
            int position = -1;
            while (stream.incrementToken()) {
                position += increment.getPositionIncrement();
                String encoded = Base64.getEncoder().encodeToString(term.toString().getBytes(StandardCharsets.UTF_8));
                System.out.printf("%d\t%d\t%d\t%s%n", position, offsets.startOffset(), offsets.endOffset(), encoded);
            }
            stream.end();
            System.out.printf("END\t%d\t%d%n", offsets.startOffset(), offsets.endOffset());
        }
    }
}
