# rspb

rust fork of [ptpb/pb](https://pb.mgt.moe)

## TL;DR

Create a new paste from the output of cmd:

```
cmd | curl -F c=@- https://pb.mgt.moe/
```

## Creating pastes
```
> echo hi | curl -F c=@- https://pb.mgt.moe/
date: 2021-01-16 03:26:09.614299435 UTC
digest: 0b8b60248fad7ac6dfac221b7e01a8b91c772421a15b387dd1fb2d6a94aee438
short: e74l
size: 3
url: http://pb.mgt.moe/e74l
status: created
uuid: 7535e567-173f-4ba0-98ce-71cdd8f02d69
```
## Updating pastes
```
> curl -X PUT -F c=@- pb.mgt.moe/7535e567-173f-4ba0-98ce-71cdd8f02d69 < config.yaml

http://pb.mgt.moe/e74l updated
```
## Using mimetypes

Append '.pdf' to hint at browsers that they should probably display a pdf document:
```
https://pb.mgt.moe/ullp.pdf
```
## Deleting pastes
```
> curl -X DELETE pb.mgt.moe/7535e567-173f-4ba0-98ce-71cdd8f02d69

deleted 7535e567-173f-4ba0-98ce-71cdd8f02d69
```
## Shortening URLs

```
> echo google.com | curl -F c=@- pb.mgt.moe/u
date: 2021-01-16 03:29:13.865511999 UTC
digest: a1adc32c271516bfb33069304087db349649146f24744b4028d2f975697fd707
short: 1unf
size: 11
url: http://pb.mgt.moe/1unf
status: created
uuid: b87dcc37-a4c2-4d18-a3a3-c2d875912cde
```

## Syntax highlighting

add '.rs' to the url to highlight rust source

```
http://pb.mgt.moe/1e6d.rs
```

## Vanity pastes

```
> echo nin | curl -F c=@- https://pb.mgt.moe/mom
date: 2021-01-16 03:34:22.359830934 UTC
digest: e2f55e5ed88dee2a50c9bb255ad87657e7f173e2560e27ceec8b206e2bc4afaf
short: 20ko
size: 4
url: http://pb.mgt.moe/mom
status: created
uuid: bac23f0c-0f06-4525-8ae4-624268485ef7
```

## Sunsetting pastes

```
> echo "This message will self-destruct in 5 seconds" | curl -F sunset=5 -F c=@- pb.mgt.moe
date: 2021-01-16 03:32:33.225306167 UTC
digest: 15cefec0e22ce1b1bfc1d06c77620cc41f8d6f1664edb023a8d63b5d0b6ef5a7
short: 19vl
size: 45
url: http://pb.mgt.moe/19vl
status: created
uuid: 7ede8735-7af3-4ee7-87bb-fc63d2a39306
> curl http://pb.mgt.moe/19vl
This message will self-destruct in 5 seconds
> sleep 5
> curl http://pb.mgt.moe/19vl
expired
```
