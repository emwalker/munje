image:
	docker-compose build
	docker tag emwalker/munje:latest emwalker/munje:$(shell cat k8s/release)

push:
	docker push emwalker/munje:$(shell cat k8s/release)

deploy:
	kubectl apply -f k8s/cluster/frontend.yaml

start:
	docker-compose --env-file=.env.docker.local up munje

proxy:
	kubectl port-forward --namespace default svc/postgres-postgresql 5433:5432

save-production:
	./scripts/save-production-db
