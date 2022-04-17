build:
	docker-compose build
	docker tag emwalker/munje:latest emwalker/munje:$(shell cat k8s/release)

push:
	docker push emwalker/munje:$(shell cat k8s/release)

deploy:
	kubectl apply -f k8s/cluster/frontend.yaml

start-prod:
	docker-compose --env-file=.env.docker.local up munje

start:
	cargo run

proxy:
	kubectl port-forward --namespace default svc/postgres-postgresql 5433:5432

save-production:
	./scripts/save-production-db

test:
	cargo test

format:
	cargo fmt
