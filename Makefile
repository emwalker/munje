image:
	docker-compose build

start:
	docker-compose --env-file=.env.docker.local up munje
