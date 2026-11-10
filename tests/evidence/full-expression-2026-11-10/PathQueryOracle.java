// SPDX-License-Identifier: MIT
// Copyright (c) 2026 swchen44
// Independent correctness adapter; links the unchanged original OpenGrok classes.
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import org.apache.lucene.search.Query;
import org.opengrok.indexer.search.QueryBuilder;

public class PathQueryOracle {
    public static void main(String[] args) {
        for (String encoded : args) {
            String input = new String(Base64.getDecoder().decode(encoded), StandardCharsets.UTF_8);
            try {
                Query parsed = new QueryBuilder().setFreetext(input).build();
                System.out.println(encoded + "\tOK\t" + (parsed == null ? "<null>" : parsed.toString()));
            } catch (Exception e) {
                System.out.println(encoded + "\tERROR\t" + e.getClass().getName() + "\t" + e.getMessage().replace('\n', ' '));
            }
        }
    }
}
