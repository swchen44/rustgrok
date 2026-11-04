import java.nio.file.Path;
import org.apache.lucene.index.DirectoryReader;
import org.apache.lucene.store.FSDirectory;
import org.apache.lucene.search.IndexSearcher;
import org.apache.lucene.search.MatchAllDocsQuery;
public class RustgrokIndexInventory {
 public static void main(String[] args) throws Exception {
  try (var reader=DirectoryReader.open(FSDirectory.open(Path.of(args[0])))) {
   var searcher=new IndexSearcher(reader);
   var hits=searcher.search(new MatchAllDocsQuery(),Math.max(1,reader.maxDoc()));
   System.out.printf("maxDoc=%d numDocs=%d hits=%d%n",reader.maxDoc(),reader.numDocs(),hits.totalHits.value);
   for (var hit:hits.scoreDocs) {
    var doc=searcher.storedFields().document(hit.doc);
    System.out.printf("doc=%d path=%s%n",hit.doc,doc.get("path"));
   }
  }
 }
}
