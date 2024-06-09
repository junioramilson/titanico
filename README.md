# Titanico

A high-performance MongoDB connection manager built with Rust. Titanico is particularly well-suited for applications using AWS Lambda functions that require frequent and concurrent connections to MongoDB. By centralizing and managing the connections efficiently, Titanico significantly improves connection reuse, reduces the overhead associated with cold starts, and ensures optimal resource utilization.

## Roadmap

- CloudFormation template for deployment in AWS.
- Improve body request validation.
- Health endpoint.
- Docker setup.
- Support for all MongoDB Operations with all Filters and Options.
- Min and Max pool size configurable via Environment Variable.
- NodeJS Client SDK.
