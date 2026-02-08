GRAPH.QUERY DEMO_GRAPH "MATCH p=(charlie:Actor { name: 'Charlie Sheen' })-[:PLAYED_WITH*1..3]->(:Actor)
RETURN nodes(p) as actors"
