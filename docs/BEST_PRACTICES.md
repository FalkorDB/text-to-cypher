# Text-to-Cypher Best Practices

This guide provides best practices for using and optimizing the text-to-cypher system based on current research and industry standards.

## Table of Contents

1. [Prompt Engineering](#prompt-engineering)
2. [Schema Design](#schema-design)
3. [Query Optimization](#query-optimization)
4. [Error Handling](#error-handling)
5. [Production Deployment](#production-deployment)
6. [Monitoring and Maintenance](#monitoring-and-maintenance)

## Prompt Engineering

### Writing Effective Natural Language Queries

#### DO ✅

**Be Specific and Clear**
```
Good: "Find all customers who purchased more than $1000 worth of products in 2024"
Bad: "Show me customers"
```

**Use Domain Terminology**
```
Good: "Which actors appeared in movies directed by Christopher Nolan?"
Bad: "People in things made by that guy"
```

**Specify Desired Output**
```
Good: "Return the names and ages of all employees in the Engineering department"
Bad: "Get engineering people"
```

**Include Constraints**
```
Good: "Find products with price between $50 and $100 sorted by rating"
Bad: "Find products"
```

#### DON'T ❌

**Avoid Ambiguous References**
```
Bad: "Show me the ones from yesterday"
Good: "Show me orders created on 2024-01-15"
```

**Don't Mix Multiple Questions**
```
Bad: "Show me customers and their orders and products they bought and the reviews"
Good: "Show me customers with their orders and the products in each order"
```

**Avoid Vague Quantifiers**
```
Bad: "Find people with lots of friends"
Good: "Find people with more than 10 connections"
```

### System Prompt Optimization

The system prompt in `templates/system_prompt.txt` follows these principles:

1. **Clear Task Definition**: Explicitly states the task is to generate Cypher
2. **Schema Context**: Includes the ontology/schema in the prompt
3. **Constraints**: Lists what the model MUST and MUST NOT do
4. **Examples**: Provides representative examples
5. **Validation Checklist**: Guides the model through validation steps

## Schema Design

### Best Practices for Graph Schemas

#### 1. Consistent Naming Conventions

```cypher
// Good: Consistent PascalCase for labels
(:Person)-[:KNOWS]->(:Person)
(:Company)-[:EMPLOYS]->(:Person)

// Bad: Inconsistent naming
(:person)-[:knows]->(:PERSON)
(:company)-[:Employs]->(:Person)
```

#### 2. Descriptive Relationship Types

```cypher
// Good: Specific relationship types
(:Person)-[:WORKS_FOR]->(:Company)
(:Person)-[:MANAGES]->(:Department)

// Bad: Generic relationships
(:Person)-[:RELATED_TO]->(:Company)
(:Person)-[:HAS]->(:Department)
```

#### 3. Meaningful Property Names

```cypher
// Good: Clear property names
CREATE (p:Person {
    firstName: "John",
    lastName: "Doe",
    dateOfBirth: date("1990-01-01"),
    email: "john.doe@example.com"
})

// Bad: Cryptic property names
CREATE (p:Person {
    fn: "John",
    ln: "Doe",
    dob: "1990-01-01",
    em: "john.doe@example.com"
})
```

#### 4. Property Value Consistency

```cypher
// Good: Consistent value formats
CREATE (p:Person {name: "John Doe"})
CREATE (p:Person {name: "Jane Smith"})

// Bad: Inconsistent formats
CREATE (p:Person {name: "John Doe"})
CREATE (p:Person {name: "JANE SMITH"})
CREATE (p:Person {name: "bob-jones"})
```

### Schema Enhancement Tips

1. **Add Example Values**: The system now collects examples automatically
2. **Document Value Ranges**: Use constraints or properties to define valid ranges
3. **Use Indexes**: Create indexes on frequently queried properties
4. **Normalize Names**: Store canonical forms and use `toLower()` for matching

```cypher
// Create index for better performance
CREATE INDEX person_name_index FOR (p:Person) ON (p.name)

// Create constraint for data quality
CREATE CONSTRAINT person_email_unique FOR (p:Person) REQUIRE p.email IS UNIQUE
```

## Query Optimization

### Writing Efficient Natural Language Queries

#### 1. Limit Result Sets

```
Good: "Find the top 10 highest-rated movies"
Average: "Find highly-rated movies" (might return too many)
```

#### 2. Use Specific Filters

```
Good: "Find customers in California who made purchases in 2024"
Average: "Find customers who made purchases"
```

#### 3. Avoid Overly Complex Single Queries

```
Bad: "Find all customers, their orders, products, suppliers, reviews, and related recommendations with detailed analytics"

Good: Break into multiple queries:
1. "Find customers with orders in 2024"
2. "For customer John Doe, show order details and products"
3. "Show reviews for product X"
```

### Understanding Generated Queries

The system generates Cypher based on your schema. Here's what to expect:

**Simple Entity Lookup**
```
Input: "Find person named John"
Output: MATCH (p:Person) WHERE toLower(p.name) = 'john' RETURN p
```

**Relationship Traversal**
```
Input: "Who are John's friends?"
Output: MATCH (p:Person {name: 'John'})-[:KNOWS]->(friend:Person) RETURN friend
```

**Aggregation**
```
Input: "How many orders does each customer have?"
Output: MATCH (c:Customer)-[:PLACED]->(o:Order) RETURN c.name, count(o) AS orderCount
```

## Error Handling

### Common Errors and Solutions

#### 1. Property Not Found

**Error**: `Property 'name' not found on node type 'Person'`

**Solutions**:
- Check schema for correct property name
- Verify data exists with examples
- Use `WHERE EXISTS(n.property)` to check

#### 2. Validation Errors

The system now validates queries before execution. Common validation errors:

**Unbalanced Parentheses**
```
Bad: MATCH (p:Person WHERE p.name = 'John' RETURN p
Fixed: MATCH (p:Person) WHERE p.name = 'John' RETURN p
```

**Missing RETURN Clause**
```
Bad: MATCH (p:Person) WHERE p.age > 30
Fixed: MATCH (p:Person) WHERE p.age > 30 RETURN p
```

#### 3. Self-Healing

When a query fails, the system automatically attempts to fix it:

1. **Validation Failures**: Regenerates with validation errors as context
2. **Execution Failures**: Regenerates with execution error feedback
3. **Fallback**: Reports error if self-healing fails

### Handling Failed Queries

If self-healing fails:

1. **Review the Schema**: Ensure your graph matches the expected schema
2. **Simplify the Query**: Try a simpler, more specific question
3. **Check Examples**: Verify example values match your data
4. **Clear Cache**: Use `/clear_schema_cache/{graph_name}` if schema changed

## Production Deployment

### Configuration Best Practices

#### 1. Environment Variables

```bash
# Required
DEFAULT_MODEL=gpt-4o-mini
DEFAULT_KEY=your-api-key

# Optional but recommended
FALKORDB_CONNECTION=falkor://127.0.0.1:6379
REST_PORT=8080
MCP_PORT=3001
```

#### 2. Schema Caching

The system caches schemas for performance. Consider:

- **Cache Size**: Default is 100 graphs (configurable)
- **Cache Invalidation**: Use `/clear_schema_cache` when schema changes
- **Cold Start**: First query per graph discovers schema (slower)

#### 3. Rate Limiting

Implement rate limiting at the API level:

```bash
# Using nginx
limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
limit_req zone=api burst=20;
```

#### 4. Model Selection

Choose models based on your requirements:

| Model | Speed | Accuracy | Cost | Use Case |
|-------|-------|----------|------|----------|
| gpt-4o-mini | Fast | Good | Low | Development, simple queries |
| gpt-4o | Medium | Better | Medium | Production, complex queries |
| gpt-4 | Slow | Best | High | Critical queries, complex schemas |

### Security Considerations

#### 1. Query Validation

The validator checks for dangerous operations:
- `DROP` statements
- `DELETE` without constraints
- Unbounded operations

#### 2. Input Sanitization

Always validate user input:
```rust
// The system does this automatically
let validation = CypherValidator::validate(query);
if !validation.is_valid {
    return Error("Invalid query");
}
```

#### 3. API Key Management

- Never commit API keys to version control
- Use environment variables or secrets management
- Rotate keys regularly
- Use separate keys for dev/staging/prod

#### 4. Access Control

Implement at the API level:
```rust
// Example middleware
async fn auth_middleware(req: Request) -> Result<Request> {
    verify_api_token(req.headers().get("Authorization"))?;
    Ok(req)
}
```

## Monitoring and Maintenance

### Key Metrics to Track

#### 1. Query Success Rate

Monitor the percentage of successful query executions:
```
success_rate = successful_queries / total_queries
```

**Target**: >95% success rate

#### 2. Self-Healing Effectiveness

Track how often self-healing succeeds:
```
self_healing_rate = queries_fixed_by_healing / failed_queries
```

**Target**: >60% of failures fixed

#### 3. Query Latency

Monitor end-to-end latency:
- Schema discovery: <500ms (cached) / <5s (uncached)
- Query generation: <2s (simple) / <5s (complex)
- Query execution: <100ms (simple) / <1s (complex)
- Total: <3s (simple) / <10s (complex)

#### 4. Validation Failures

Track validation failure types:
- Syntax errors
- Dangerous operations
- Missing clauses
- Unbalanced syntax

### Logging Best Practices

Enable structured logging:

```rust
tracing::info!(
    query = %cypher_query,
    graph = %graph_name,
    duration_ms = %duration.as_millis(),
    "Query executed successfully"
);
```

### Regular Maintenance

#### 1. Schema Updates

When your graph schema changes:
```bash
# Clear cache for specific graph
curl -X POST http://localhost:8080/clear_schema_cache/my_graph

# Or clear entire cache by restarting service
docker restart text-to-cypher
```

#### 2. Model Updates

When switching models:
1. Test with sample queries first
2. Monitor success rates
3. Adjust system prompts if needed
4. Roll back if issues occur

#### 3. Example Value Refresh

Example values are collected during schema discovery. To refresh:
1. Clear schema cache
2. Next query will rediscover schema with new examples

### Performance Tuning

#### 1. Schema Discovery

Adjust sample size based on data volume:
```rust
// Default is 100, increase for better examples
Schema::discover_from_graph(&mut graph, 200).await
```

#### 2. Concurrent Requests

The system uses async processing. Configure based on load:
- CPU-bound: Number of cores
- I/O-bound: Higher (10x cores)

#### 3. Database Connection Pooling

Configure FalkorDB connection pool:
```rust
FalkorClientBuilder::new_async()
    .with_max_connections(10)
    .build()
```

## Testing Strategies

### 1. Unit Testing

Test individual components:
```rust
#[test]
fn test_query_validation() {
    let query = "MATCH (n:Person) RETURN n";
    let result = CypherValidator::validate(query);
    assert!(result.is_valid);
}
```

### 2. Integration Testing

Test end-to-end flows:
```bash
# Test query generation
curl -X POST http://localhost:8080/text_to_cypher \
  -H "Content-Type: application/json" \
  -d '{
    "graph_name": "test",
    "chat_request": {
      "messages": [{"role": "user", "content": "Find all persons"}]
    }
  }'
```

### 3. Schema Testing

Verify schema discovery:
```bash
curl http://localhost:8080/get_schema/test_graph
```

### 4. Load Testing

Use tools like Apache Bench or k6:
```bash
ab -n 100 -c 10 http://localhost:8080/text_to_cypher
```

## Troubleshooting

### Common Issues

#### Issue: Slow Query Generation

**Possible Causes**:
- Large schema
- Complex question
- Slow LLM response

**Solutions**:
- Reduce schema sample size
- Simplify question
- Use faster model
- Implement request timeout

#### Issue: Inaccurate Queries

**Possible Causes**:
- Unclear question
- Missing schema information
- Insufficient examples

**Solutions**:
- Rephrase question more clearly
- Ensure schema is up to date
- Add more example values
- Use better model

#### Issue: High Failure Rate

**Possible Causes**:
- Schema mismatch
- Data quality issues
- Model hallucination

**Solutions**:
- Verify schema matches data
- Improve data consistency
- Enable validation and self-healing
- Use better model

## Resources

### Documentation
- [Main README](../readme.md)
- [Improvements Guide](./IMPROVEMENTS.md)
- [Docker Release Guide](./DOCKER_RELEASE.md)

### External Resources
- [Neo4j Cypher Manual](https://neo4j.com/docs/cypher-manual/)
- [FalkorDB Documentation](https://docs.falkordb.com/)
- [Text2Cypher Research Paper](https://arxiv.org/abs/2412.10064)

### Community
- [GitHub Issues](https://github.com/FalkorDB/text-to-cypher/issues)
- [FalkorDB Discord](https://discord.gg/falkordb)

## Contributing

Contributions are welcome! When contributing:

1. Follow these best practices in your code
2. Add tests for new features
3. Update documentation
4. Consider backward compatibility

## Conclusion

Following these best practices will help you:
- Generate more accurate queries
- Achieve better performance
- Handle errors gracefully
- Deploy reliably to production
- Maintain the system effectively

For questions or issues, please open a GitHub issue or reach out to the community.
