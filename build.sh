#!/bin/bash

wash build &
wash build -p bad-janet &
wash build -p good-janet &