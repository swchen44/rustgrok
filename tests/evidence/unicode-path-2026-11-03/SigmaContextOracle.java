// Independent correctness-only wrapper around the pinned OpenGrok analyzer.
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import org.opengrok.indexer.analysis.CompatibleAnalyser;
import org.apache.lucene.analysis.TokenStream;
import org.apache.lucene.analysis.tokenattributes.CharTermAttribute;
import org.apache.lucene.analysis.tokenattributes.OffsetAttribute;
import org.apache.lucene.analysis.tokenattributes.PositionIncrementAttribute;

public class SigmaContextOracle {
    private static String encode(String value) {
        return Base64.getEncoder().encodeToString(value.getBytes(StandardCharsets.UTF_8));
    }
    public static void main(String[] args) throws Exception {
        List<String> sides = new ArrayList<>();
        sides.add("");
        for (char c = 'A'; c <= 'Z'; c++) sides.add(String.valueOf(c));
        for (char c = 'a'; c <= 'z'; c++) sides.add(String.valueOf(c));
        for (String s : new String[]{"İ", "Α", "α", "Ο", "ο", "σ", "ς"}) sides.add(s);
        try (CompatibleAnalyser analyzer = new CompatibleAnalyser()) {
            for (String left : sides) for (String right : sides) {
                String input = left + "Σ" + right;
                try (TokenStream stream = analyzer.tokenStream("path", input)) {
                    CharTermAttribute term = stream.addAttribute(CharTermAttribute.class);
                    OffsetAttribute offset = stream.addAttribute(OffsetAttribute.class);
                    PositionIncrementAttribute increment = stream.addAttribute(PositionIncrementAttribute.class);
                    stream.reset();
                    if (!stream.incrementToken()) throw new IllegalStateException("missing token");
                    int position = -1 + increment.getPositionIncrement();
                    System.out.printf("%s\t%d\t%d\t%d\t%s%n", encode(input), position,
                            offset.startOffset(), offset.endOffset(), encode(term.toString()));
                    if (stream.incrementToken()) throw new IllegalStateException("unexpected second token");
                    stream.end();
                }
            }
        }
    }
}
