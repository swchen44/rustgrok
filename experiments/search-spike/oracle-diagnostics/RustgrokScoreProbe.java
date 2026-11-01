import java.nio.file.Path;
import java.util.*;
import org.apache.lucene.index.*;
import org.apache.lucene.store.FSDirectory;
import org.apache.lucene.search.*;
import org.apache.lucene.document.Document;
import org.opengrok.indexer.search.QueryBuilder;

public class RustgrokScoreProbe {
  public static void main(String[] a) throws Exception {
    List<IndexReader> readers = new ArrayList<>();
    for (int i=2;i<a.length;i++) readers.add(DirectoryReader.open(FSDirectory.open(Path.of(a[0], "index", a[i]))));
    try (MultiReader reader = new MultiReader(readers.toArray(new IndexReader[0]))) {
      IndexSearcher searcher = new IndexSearcher(reader);
      QueryBuilder builder = new QueryBuilder();
      Query query = a[1].startsWith("path=") ? builder.setPath(a[1].substring(5)).build() : switch (a[1]) {
        case "full" -> builder.setFreetext("main").build();
        case "def" -> builder.setDefs("main").build();
        case "symbol" -> builder.setRefs("main").build();
        case "path" -> builder.setPath("main.c").build();
        default -> throw new IllegalArgumentException();
      };
      System.out.println("query=" + query + " type=" + query.getClass().getName() + " maxDoc="+reader.maxDoc());
      TopDocs results = searcher.search(query, new TopScoreDocCollectorManager(Math.max(1,reader.maxDoc()), Integer.MAX_VALUE));
      for (ScoreDoc hit : results.scoreDocs) {
        Document doc = searcher.storedFields().document(hit.doc);
        System.out.println("doc="+hit.doc+" score="+Float.toHexString(hit.score)+" fullpath="+doc.get("fullpath")+" path="+doc.get("path"));
        System.out.println(searcher.explain(query, hit.doc));
      }
    }
  }
}
